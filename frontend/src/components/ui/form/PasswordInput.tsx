import type { Component, JSX } from "solid-js";
import { createSignal, Show, splitProps } from "solid-js";
import type { Size } from "~/components/ui/layout/layout";
import { cn } from "~/lib/utils";

export type PasswordInputSize = Extract<Size, "sm" | "md">;

export interface PasswordInputProps
  extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "type"> {
  size?: PasswordInputSize;
  startIcon?: JSX.Element;
  endIcon?: JSX.Element;
}

const sizeMap: Record<PasswordInputSize, string> = {
  sm: "input-sm",
  md: "",
};

export const PasswordInput: Component<PasswordInputProps> = (props) => {
  const [local, rest] = splitProps(props, [
    "size",
    "startIcon",
    "endIcon",
    "class",
    "classList",
    "disabled",
  ]);
  const [visible, setVisible] = createSignal(false);

  const toggle = () => setVisible((v) => !v);

  return (
    <label
      class={cn(
        "input input-bordered w-full",
        sizeMap[local.size ?? "md"],
        local.class,
      )}
      classList={local.classList}
    >
      {local.startIcon}
      <input
        {...rest}
        type={visible() ? "text" : "password"}
        disabled={local.disabled}
        class="grow"
      />
      <button
        type="button"
        class="shrink-0 border-0 bg-transparent p-0"
        onClick={(e) => {
          e.preventDefault();
          toggle();
        }}
        disabled={local.disabled}
        aria-label={visible() ? "Hide password" : "Show password"}
      >
        <Show
          when={visible()}
          fallback={
            <svg
              class="size-[1em] opacity-50 transition-opacity hover:opacity-100 active:opacity-70"
              fill="currentColor"
              aria-hidden="true"
            >
              <use href="/assets/icons.svg#icon-visibility" />
            </svg>
          }
        >
          <svg
            class="size-[1em] opacity-50 transition-opacity hover:opacity-100 active:opacity-70"
            fill="currentColor"
            aria-hidden="true"
          >
            <use href="/assets/icons.svg#icon-visibility_off" />
          </svg>
        </Show>
      </button>
      {local.endIcon}
    </label>
  );
};
