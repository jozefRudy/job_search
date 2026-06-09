import type { Component, JSX } from "solid-js";
import { splitProps } from "solid-js";
import { cn } from "~/lib/utils";

export interface SidebarLayoutProps extends JSX.HTMLAttributes<HTMLDivElement> {
  sidebar: JSX.Element;
}

export const SidebarLayout: Component<SidebarLayoutProps> = (props) => {
  const [local, rest] = splitProps(props, ["sidebar", "children", "class"]);
  return (
    <div {...rest} class={cn("drawer lg:drawer-open", local.class)}>
      <input id="sidebar-drawer" type="checkbox" class="drawer-toggle" />
      <div class="drawer-content flex min-h-screen flex-col">
        <header class="navbar bg-base-100 lg:hidden">
          <div class="navbar-start">
            <label
              for="sidebar-drawer"
              class="btn btn-sm btn-ghost drawer-button"
              aria-label="Open sidebar"
            >
              <svg class="h-5 w-5" fill="currentColor" aria-hidden="true">
                <use href="/assets/icons.svg#icon-hamburger" />
              </svg>
            </label>
          </div>
        </header>
        <main class="flex-1">{local.children}</main>
      </div>
      <div class="drawer-side z-40">
        <label
          for="sidebar-drawer"
          aria-label="close sidebar"
          class="drawer-overlay"
        />
        {local.sidebar}
      </div>
    </div>
  );
};
