export type Gap = "none" | "sm" | "md" | "lg";
export type Size = "sm" | "md" | "lg";
export type Align = "start" | "center" | "end" | "stretch";
export type Justify =
  | "start"
  | "center"
  | "end"
  | "between"
  | "around"
  | "evenly";

export const gapMap: Record<Gap, string> = {
  none: "",
  sm: "gap-2",
  md: "gap-4",
  lg: "gap-8",
};

export const alignMap: Record<Align, string> = {
  start: "items-start",
  center: "items-center",
  end: "items-end",
  stretch: "items-stretch",
};

export const justifyMap: Record<Justify, string> = {
  start: "justify-start",
  center: "justify-center",
  end: "justify-end",
  between: "justify-between",
  around: "justify-around",
  evenly: "justify-evenly",
};
