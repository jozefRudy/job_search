import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import type { Size } from "~/components/ui/layout/layout";
import { cn } from "~/lib/utils";

export type ButtonVariant =
  | "primary"
  | "secondary"
  | "ghost"
  | "danger"
  | "link";
export type ButtonState = "normal" | "disabled" | "loading";

export interface ButtonProps
  extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: Size;
  state?: ButtonState;
  icon?: string;
  iconPlacement?: "left" | "right";
  href?: string;
}

const variantMap: Record<ButtonVariant, string> = {
  primary: "btn-primary",
  secondary: "btn-secondary",
  ghost: "btn-ghost",
  danger: "btn-error",
  link: "btn-link",
};

const sizeMap: Record<Size, string> = {
  sm: "btn-xs",
  md: "btn-sm",
  lg: "btn-md",
};

function SpriteIcon(props: { id: string; class?: string }) {
  return (
    <svg class={props.class} fill="currentColor" aria-label={props.id}>
      <use href={`/assets/icons.svg#icon-${props.id}`} />
    </svg>
  );
}

const defaultProps = {
  variant: "primary" as ButtonVariant,
  size: "md" as Size,
  state: "normal" as ButtonState,
  iconPlacement: "left" as "left" | "right",
};

export const Button: Component<ButtonProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "variant",
    "size",
    "state",
    "icon",
    "iconPlacement",
    "href",
    "children",
    "class",
    "classList",
  ]);

  const disabled = () =>
    local.state === "disabled" || local.state === "loading";

  const classes = () =>
    cn(
      "btn",
      variantMap[local.variant],
      sizeMap[local.size],
      disabled() && "btn-disabled",
      local.class,
    );

  const loadingSizeMap: Record<Size, string> = {
    sm: "loading-xs",
    md: "loading-xs",
    lg: "loading-sm",
  };

  const iconContent = () => {
    if (local.state === "loading") {
      return (
        <span
          class={cn("loading loading-spinner", loadingSizeMap[local.size])}
        />
      );
    }
    if (local.icon) {
      return <SpriteIcon id={local.icon} class="size-[1em]" />;
    }
    return null;
  };

  const inner = (
    <>
      {local.iconPlacement === "left" && iconContent()}
      {local.children}
      {local.iconPlacement === "right" && iconContent()}
    </>
  );

  if (local.href) {
    return (
      <a href={local.href} class={classes()} classList={local.classList}>
        {inner}
      </a>
    );
  }

  return (
    <button
      {...rest}
      disabled={disabled()}
      class={classes()}
      classList={local.classList}
    >
      {inner}
    </button>
  );
};
