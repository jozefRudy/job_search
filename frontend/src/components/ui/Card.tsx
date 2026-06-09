import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import type { Size } from "~/components/ui/layout/layout";
import { cn } from "~/lib/utils";

export type CardBehavior = "static" | "hoverable" | "clickable";

export interface CardProps extends JSX.HTMLAttributes<HTMLDivElement> {
  size?: Size;
  behavior?: CardBehavior;
}

const sizeMap: Record<Size, string> = {
  sm: "card-sm",
  md: "",
  lg: "card-lg",
};

const defaultProps = {
  size: "md" as Size,
  behavior: "static" as CardBehavior,
};

export const Card: Component<CardProps> & {
  Title: Component<JSX.HTMLAttributes<HTMLHeadingElement>>;
  Body: Component<JSX.HTMLAttributes<HTMLDivElement>>;
  Actions: Component<JSX.HTMLAttributes<HTMLDivElement>>;
} = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "size",
    "behavior",
    "children",
    "class",
    "classList",
  ]);

  const isHoverable = () =>
    local.behavior === "hoverable" || local.behavior === "clickable";
  const isClickable = () => local.behavior === "clickable";

  const classes = () =>
    cn(
      "card bg-base-200",
      sizeMap[local.size],
      isHoverable() && "hover:bg-base-300 transition-colors",
      isClickable() &&
        "cursor-pointer active:scale-[0.98] active:brightness-95 transition-all",
      local.class,
    );

  return (
    <div {...rest} class={classes()} classList={local.classList}>
      {local.children}
    </div>
  );
};

Card.Title = (props) => {
  const [local, rest] = splitProps(props, ["children", "class"]);
  return (
    <h2 {...rest} class={cn("card-title", local.class)}>
      {local.children}
    </h2>
  );
};

Card.Body = (props) => {
  const [local, rest] = splitProps(props, ["children", "class"]);
  return (
    <div {...rest} class={cn("card-body", local.class)}>
      {local.children}
    </div>
  );
};

Card.Actions = (props) => {
  const [local, rest] = splitProps(props, ["children", "class"]);
  return (
    <div {...rest} class={cn("card-actions", local.class)}>
      {local.children}
    </div>
  );
};
