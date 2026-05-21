import { customType } from "drizzle-orm/pg-core";

export const vector = (dimensions: number) =>
  customType<{ data: number[]; driverData: string }>({
    dataType() {
      return `vector(${dimensions})`;
    },
    toDriver(value: number[]): string {
      return `[${value.join(",")}]`;
    },
    fromDriver(value: string): number[] {
      const cleaned = value.replace(/^\[/, "").replace(/\]$/, "");
      if (!cleaned) return [];
      return cleaned.split(",").map(Number);
    },
  });
