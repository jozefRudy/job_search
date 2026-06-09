import type { Component } from "solid-js";
import { createSignal, onMount } from "solid-js";

export const ThemeToggle: Component = () => {
  const [theme, setTheme] = createSignal<"business" | "corporate">("business");

  onMount(() => {
    const saved = document.documentElement.getAttribute("data-theme");
    if (saved === "corporate") setTheme("corporate");
  });

  const toggle = () => {
    const next = theme() === "business" ? "corporate" : "business";
    document.documentElement.setAttribute("data-theme", next);
    setTheme(next);
  };

  return (
    <button
      type="button"
      onClick={toggle}
      class="btn btn-sm btn-ghost gap-2"
      aria-label="Toggle theme"
    >
      {theme() === "business" ? "🌙 Dark" : "☀️ Light"}
    </button>
  );
};
