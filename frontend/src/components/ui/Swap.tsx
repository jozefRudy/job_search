import type { Component, JSX } from "solid-js";
import { splitProps } from "solid-js";
import type { Size } from "~/components/ui/layout/layout";
import { cn } from "~/lib/utils";

export interface SwapProps
  extends Omit<JSX.HTMLAttributes<HTMLLabelElement>, "onChange"> {
  checked: boolean;
  onChange: (checked: boolean) => void;
  on: JSX.Element;
  off: JSX.Element;
  size?: Size;
}

const sizeMap: Record<Size, string> = {
  sm: "text-sm",
  md: "text-base",
  lg: "text-lg",
};

export const Swap: Component<SwapProps> = (props) => {
  const [local, rest] = splitProps(props, [
    "checked",
    "onChange",
    "on",
    "off",
    "size",
    "class",
    "classList",
    "children",
  ]);

  const classes = () => cn("swap", sizeMap[local.size ?? "md"], local.class);

  return (
    <label {...rest} class={classes()} classList={local.classList}>
      <input
        type="checkbox"
        class="swap-hidden"
        checked={local.checked}
        onChange={(e) => local.onChange(e.currentTarget.checked)}
      />
      <div class="swap-on">{local.on}</div>
      <div class="swap-off">{local.off}</div>
    </label>
  );
};
