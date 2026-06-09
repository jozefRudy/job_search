import type { Component, JSX } from "solid-js";
import { createSignal, mergeProps, splitProps } from "solid-js";
import type { Size } from "~/components/ui/layout/layout";
import { Button } from "./Button";

export interface ClipboardButtonProps
  extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  text: string;
  size?: Size;
  children?: JSX.Element;
}

const defaultProps = {
  size: "md" as Size,
};

export const ClipboardButton: Component<ClipboardButtonProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "text",
    "size",
    "children",
    "class",
    "classList",
  ]);
  const [copied, setCopied] = createSignal(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(local.text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // ignore
    }
  };

  const icon = () =>
    copied() ? (
      <svg
        class="size-[1em] text-success"
        fill="currentColor"
        aria-hidden="true"
      >
        <use href="/assets/icons.svg#icon-check" />
      </svg>
    ) : (
      <svg class="size-[1em]" fill="currentColor" aria-hidden="true">
        <use href="/assets/icons.svg#icon-content_copy" />
      </svg>
    );

  return (
    <Button
      {...rest}
      type="button"
      variant="ghost"
      size={local.size}
      class={local.class}
      classList={local.classList}
      aria-label={copied() ? "Copied" : "Copy to clipboard"}
      onClick={(e) => {
        e.preventDefault();
        copy();
      }}
    >
      {icon()}
      {local.children ?? (copied() ? "Copied" : "Copy")}
    </Button>
  );
};
