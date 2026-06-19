import type { Component, JSX } from "solid-js";

export const ErrorAlert: Component<{ children: JSX.Element }> = (props) => {
  return (
    <div class="rounded-box bg-error/10 p-4 text-error">{props.children}</div>
  );
};
