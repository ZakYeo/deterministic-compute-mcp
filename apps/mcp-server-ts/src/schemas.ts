import * as z from "zod/v4";

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

export type ArithmeticToolInput = z.infer<typeof arithmeticToolInputSchema>;
export type ExpressionToolInput = z.infer<typeof expressionToolInputSchema>;
