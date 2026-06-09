import type { ZodType } from "zod";

export function zodValidator<T>(schema: ZodType<T>) {
  return (value: unknown): string => {
    const result = schema.safeParse(value);
    if (!result.success) {
      return result.error.issues[0]?.message ?? "Invalid value";
    }
    return "";
  };
}
