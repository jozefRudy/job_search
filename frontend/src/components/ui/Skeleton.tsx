import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

export type SkeletonVariant = "block" | "circle" | "text";

export interface SkeletonProps extends JSX.HTMLAttributes<HTMLDivElement> {
  variant?: SkeletonVariant;
}

const variantMap: Record<SkeletonVariant, string> = {
  block: "",
  circle: "rounded-full",
  text: "skeleton-text",
};

const defaultProps = {
  variant: "block" as SkeletonVariant,
};

export const Skeleton: Component<SkeletonProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "variant",
    "children",
    "class",
    "classList",
  ]);

  const classes = () =>
    cn("skeleton", variantMap[local.variant], "w-full", local.class);

  return (
    <div {...rest} class={classes()} classList={local.classList}>
      {local.children}
    </div>
  );
};
