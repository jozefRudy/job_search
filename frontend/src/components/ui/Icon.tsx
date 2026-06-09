import type { Component, JSX } from "solid-js";
import { splitProps } from "solid-js";
import { cn } from "~/lib/utils";

export interface IconProps extends JSX.SvgSVGAttributes<SVGSVGElement> {
  id: string;
}

export const Icon: Component<IconProps> = (props) => {
  const [local, rest] = splitProps(props, [
    "id",
    "class",
    "classList",
    "children",
  ]);

  return (
    <svg
      {...rest}
      class={cn("size-[1em]", local.class)}
      fill="currentColor"
      aria-hidden="true"
    >
      <use href={`/assets/icons.svg#icon-${local.id}`} />
    </svg>
  );
};
