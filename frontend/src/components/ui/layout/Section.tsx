import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

export type SectionBg =
  | "base-100"
  | "base-200"
  | "base-300"
  | "primary"
  | "accent"
  | "none";
export type SectionPaddingY = "none" | "sm" | "md" | "lg";

export interface SectionProps extends JSX.HTMLAttributes<HTMLElement> {
  bg?: SectionBg;
  paddingY?: SectionPaddingY;
}

const bgMap: Record<SectionBg, string> = {
  "base-100": "bg-base-100",
  "base-200": "bg-base-200",
  "base-300": "bg-base-300",
  primary: "bg-primary",
  accent: "bg-accent",
  none: "",
};

const paddingYMap: Record<SectionPaddingY, string> = {
  none: "",
  sm: "py-8",
  md: "py-16",
  lg: "py-24",
};

const defaultProps = {
  bg: "none" as SectionBg,
  paddingY: "none" as SectionPaddingY,
};

export const Section: Component<SectionProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "bg",
    "paddingY",
    "children",
    "class",
    "classList",
  ]);

  const classes = () =>
    cn(bgMap[local.bg], paddingYMap[local.paddingY], local.class);

  return (
    <section {...rest} class={classes()} classList={local.classList}>
      {local.children}
    </section>
  );
};
