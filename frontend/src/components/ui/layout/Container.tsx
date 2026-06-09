import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

export type ContainerMaxWidth = "none" | "sm" | "md" | "lg";
export type ContainerPaddingX = "none" | "sm" | "md" | "lg";

export interface ContainerProps extends JSX.HTMLAttributes<HTMLDivElement> {
  maxWidth?: ContainerMaxWidth;
  paddingX?: ContainerPaddingX;
}

const defaultProps = {
  maxWidth: "none" as ContainerMaxWidth,
  paddingX: "none" as ContainerPaddingX,
};

const maxWidthMap: Record<ContainerMaxWidth, string> = {
  none: "max-w-full",
  sm: "max-w-screen-md",
  md: "max-w-screen-lg",
  lg: "max-w-screen-xl",
};

const paddingXMap: Record<ContainerPaddingX, string> = {
  none: "",
  sm: "px-4",
  md: "px-16",
  lg: "px-36",
};

export const Container: Component<ContainerProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "maxWidth",
    "paddingX",
    "children",
    "class",
    "classList",
  ]);

  const classes = () =>
    cn(
      "w-full mx-auto",
      maxWidthMap[local.maxWidth],
      paddingXMap[local.paddingX],
      local.class,
    );

  return (
    <div {...rest} class={classes()} classList={local.classList}>
      {local.children}
    </div>
  );
};
