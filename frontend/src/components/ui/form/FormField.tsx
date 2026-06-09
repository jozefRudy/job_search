import type { Component, JSX } from "solid-js";
import { For, Show, splitProps } from "solid-js";
import { cn } from "~/lib/utils";

export interface FormFieldProps extends JSX.HTMLAttributes<HTMLDivElement> {
  label?: string;
  hint?: string;
  error?: string[];
}

export const FormField: Component<FormFieldProps> = (props) => {
  const [local, _rest] = splitProps(props, [
    "label",
    "hint",
    "error",
    "children",
    "class",
    "classList",
  ]);

  const errors = () => local.error ?? [];

  return (
    <div class={cn("space-y-1", local.class)} classList={local.classList}>
      <Show when={local.label}>
        <span class="font-medium text-sm">{local.label}</span>
      </Show>
      {local.children}
      <Show when={errors().length === 0 && local.hint}>
        <p class="text-base-content/70 text-xs">{local.hint}</p>
      </Show>
      <Show when={errors().length > 0}>
        <div class="space-y-0.5 text-error text-xs">
          <For each={errors()}>{(msg) => <p>{msg}</p>}</For>
        </div>
      </Show>
    </div>
  );
};
