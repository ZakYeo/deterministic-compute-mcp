import * as z from "zod/v4";

const MAX_EXPECTED_VALUE_CASE_ID_BYTES = 128;
const MAX_EXPECTED_VALUE_CASE_INPUT_BYTES = 16 * 1024;
const MAX_UNIT_IDENTIFIER_CHARS = 32;

export const roundingModeSchema = z.enum([
  "exact",
  "truncate",
  "half-away-from-zero",
]);

export const precisionPolicySchema = z
  .object({
    decimalPlaces: z
      .number()
      .int()
      .min(0)
      .max(38)
      .optional()
      .describe("Required fractional decimal places for the output."),
    rounding: roundingModeSchema
      .default("exact")
      .describe("Deterministic rounding policy."),
  })
  .strict();

const cagrPrecisionPolicySchema = z
  .object({
    decimalPlaces: z
      .number()
      .int()
      .min(0)
      .max(38)
      .describe("Required fractional decimal places for the CAGR output."),
    rounding: roundingModeSchema
      .default("exact")
      .describe("Deterministic rounding policy."),
  })
  .strict();

export const numericValueSchema = z.discriminatedUnion("kind", [
  z
    .object({
      kind: z.literal("integer"),
      value: z.string().regex(/^[+-]?\d+$/),
    })
    .strict(),
  z
    .object({
      kind: z.literal("decimal"),
      value: z.string().regex(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/),
      scale: z.number().int().min(0).max(38),
    })
    .superRefine((value, context) => {
      const fractionalDigits = value.value.includes(".")
        ? (value.value.split(".")[1] ?? "").length
        : 0;

      if (fractionalDigits !== value.scale) {
        context.addIssue({
          code: "custom",
          message: "decimal value fractional digit count must match scale",
          path: ["scale"],
        });
      }
    })
    .strict(),
]);

function isNegativeNumericValue(value: z.infer<typeof numericValueSchema>): boolean {
  return value.value.trim().startsWith("-");
}

const nonNegativeNumericValueSchema = numericValueSchema.refine(
  (value) => !isNegativeNumericValue(value),
  "numeric value must be greater than or equal to zero",
);

export const arithmeticToolInputSchema = z
  .object({
    operation: z
      .enum(["add", "subtract", "multiply", "divide"])
      .describe("Arithmetic operation to compute."),
    operands: z
      .tuple([numericValueSchema, numericValueSchema])
      .describe("Exactly two JSON-safe deterministic numeric operands."),
    precision: precisionPolicySchema.optional(),
    trace: z
      .boolean()
      .default(false)
      .describe("Whether to request deterministic trace metadata."),
  })
  .strict();

export const expressionToolInputSchema = z
  .object({
    expression: z.string().min(1),
    precision: precisionPolicySchema.optional(),
    trace: z.boolean().default(false),
  })
  .strict();

export const unitConversionToolInputSchema = z
  .object({
    value: numericValueSchema,
    sourceUnit: z
      .string()
      .trim()
      .min(1)
      .max(MAX_UNIT_IDENTIFIER_CHARS),
    targetUnit: z
      .string()
      .trim()
      .min(1)
      .max(MAX_UNIT_IDENTIFIER_CHARS),
    precision: precisionPolicySchema.optional(),
    trace: z
      .boolean()
      .default(false)
      .describe("Whether to request deterministic trace metadata."),
  })
  .strict();

export const verificationToleranceSchema = z.discriminatedUnion("kind", [
  z
    .object({
      kind: z.literal("absolute"),
      value: nonNegativeNumericValueSchema.describe(
        "Non-negative absolute tolerance.",
      ),
    })
    .strict(),
  z
    .object({
      kind: z.literal("relative"),
      value: nonNegativeNumericValueSchema.describe(
        "Non-negative decimal ratio applied to abs(expected).",
      ),
    })
    .strict(),
]);

export const verificationToolInputSchema = z
  .object({
    expected: numericValueSchema,
    actual: numericValueSchema,
    tolerance: verificationToleranceSchema.optional(),
    trace: z
      .boolean()
      .default(false)
      .describe("Whether to request deterministic trace metadata."),
  })
  .strict();

export const expectedValueCaseSchema = z
  .object({
    id: z
      .string()
      .trim()
      .min(1)
      .refine(
        (value) => Buffer.byteLength(value, "utf8") <= MAX_EXPECTED_VALUE_CASE_ID_BYTES,
        `case id must be at most ${MAX_EXPECTED_VALUE_CASE_ID_BYTES} UTF-8 bytes`,
      ),
    operation: z
      .enum([
        "arithmetic.add",
        "arithmetic.subtract",
        "arithmetic.multiply",
        "arithmetic.divide",
        "expression.evaluate",
        "finance.simple-interest",
        "finance.compound-interest",
        "finance.loan-payment",
        "finance.vat",
        "finance.percentage-change",
        "finance.margin-markup",
        "finance.cagr",
        "units.convert",
        "verification.compare",
      ])
      .describe("Supported compute operation used to generate this expected value."),
    input: z
      .record(z.string(), z.unknown())
      .refine(
        (value) =>
          Buffer.byteLength(JSON.stringify(value), "utf8") <=
          MAX_EXPECTED_VALUE_CASE_INPUT_BYTES,
        `case input must serialize to at most ${MAX_EXPECTED_VALUE_CASE_INPUT_BYTES} bytes`,
      ),
    precision: precisionPolicySchema.optional(),
    trace: z.boolean().default(false),
  })
  .strict();

export const testGenerationToolInputSchema = z
  .object({
    cases: z
      .array(expectedValueCaseSchema)
      .min(1)
      .max(100)
      .describe("Deterministic cases evaluated in input order."),
    failOnCaseError: z
      .boolean()
      .default(false)
      .describe("When true, any failing generated case fails the whole request."),
    maxCases: z
      .number()
      .int()
      .min(1)
      .max(100)
      .optional()
      .describe("Optional caller-specified limit no greater than 100."),
    trace: z
      .boolean()
      .default(false)
      .describe("Whether to request deterministic generator trace metadata."),
  })
  .strict()
  .superRefine((value, context) => {
    if (value.maxCases !== undefined && value.cases.length > value.maxCases) {
      context.addIssue({
        code: "custom",
        message: "case count must not exceed maxCases",
        path: ["cases"],
      });
    }
  });

const financeCommonFields = {
  precision: precisionPolicySchema.optional(),
  trace: z
    .boolean()
    .default(false)
    .describe("Whether to request deterministic trace metadata."),
};

export const financeToolInputSchema = z.discriminatedUnion("operation", [
  z
    .object({
      operation: z.literal("simple-interest"),
      principal: numericValueSchema,
      periodicRate: numericValueSchema.describe(
        "Decimal rate per period, not a percentage whole number.",
      ),
      periods: z.number().int().min(0),
      ...financeCommonFields,
    })
    .strict(),
  z
    .object({
      operation: z.literal("compound-interest"),
      principal: numericValueSchema,
      periodicRate: numericValueSchema.describe("Decimal rate per compounding period."),
      periods: z.number().int().min(0),
      ...financeCommonFields,
    })
    .strict(),
  z
    .object({
      operation: z.literal("loan-payment"),
      principal: numericValueSchema,
      periodicRate: numericValueSchema.describe("Decimal rate per payment period."),
      periods: z.number().int().min(1),
      ...financeCommonFields,
    })
    .strict(),
  z
    .object({
      operation: z.literal("vat"),
      netAmount: nonNegativeNumericValueSchema,
      vatRate: nonNegativeNumericValueSchema.describe(
        "Decimal VAT rate, not a percentage whole number.",
      ),
      ...financeCommonFields,
    })
    .strict(),
  z
    .object({
      operation: z.literal("percentage-change"),
      oldValue: numericValueSchema,
      newValue: numericValueSchema,
      ...financeCommonFields,
    })
    .strict(),
  z
    .object({
      operation: z.literal("margin-markup"),
      cost: numericValueSchema,
      revenue: numericValueSchema,
      ...financeCommonFields,
    })
    .strict(),
  z
    .object({
      operation: z.literal("cagr"),
      beginningValue: numericValueSchema,
      endingValue: numericValueSchema,
      periods: z.number().int().min(1),
      precision: cagrPrecisionPolicySchema.describe(
        "Required. CAGR supports only exact roots representable at decimalPlaces; non-exact roots return precision errors.",
      ),
      trace: z
        .boolean()
        .default(false)
        .describe("Whether to request deterministic trace metadata."),
    })
    .strict(),
]);

export type ArithmeticToolInput = z.infer<typeof arithmeticToolInputSchema>;
export type ExpressionToolInput = z.infer<typeof expressionToolInputSchema>;
export type FinanceToolInput = z.infer<typeof financeToolInputSchema>;
export type UnitConversionToolInput = z.infer<
  typeof unitConversionToolInputSchema
>;
export type TestGenerationToolInput = z.infer<
  typeof testGenerationToolInputSchema
>;
export type VerificationToolInput = z.infer<typeof verificationToolInputSchema>;
