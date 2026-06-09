import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";
import type { Align, Gap, Justify } from "./layout";
import { alignMap, gapMap, justifyMap } from "./layout";

export interface StackProps
  extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "align"> {
  gap?: Gap;
  align?: Align;
  justify?: Justify;
}

const defaultProps = {
  gap: "none" as Gap,
  align: "stretch" as Align,
  justify: "start" as Justify,
};

export const Stack: Component<StackProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "gap",
    "align",
    "justify",
    "children",
    "class",
    "classList",
  ]);

  const classes = () =>
    cn(
      "flex flex-col",
      gapMap[local.gap],
      alignMap[local.align],
      justifyMap[local.justify],
      local.class,
    );

  return (
    <div {...rest} class={classes()} classList={local.classList}>
      {local.children}
    </div>
  );
};
