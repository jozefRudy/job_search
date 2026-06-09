import type { Component, JSX } from "solid-js";
import { mergeProps, splitProps } from "solid-js";
import { cn } from "~/lib/utils";
import type { Gap } from "./layout";
import { gapMap } from "./layout";

export type Cols = 1 | 2 | 3 | 4 | 5 | 6;

export interface GridProps extends JSX.HTMLAttributes<HTMLDivElement> {
  cols: Cols;
  mdCols?: Cols;
  lgCols?: Cols;
  gap?: Gap;
}

const colClassMap: Record<Cols, string> = {
  1: "grid-cols-1",
  2: "grid-cols-2",
  3: "grid-cols-3",
  4: "grid-cols-4",
  5: "grid-cols-5",
  6: "grid-cols-6",
};

const mdColClassMap: Record<Cols, string> = {
  1: "md:grid-cols-1",
  2: "md:grid-cols-2",
  3: "md:grid-cols-3",
  4: "md:grid-cols-4",
  5: "md:grid-cols-5",
  6: "md:grid-cols-6",
};

const lgColClassMap: Record<Cols, string> = {
  1: "lg:grid-cols-1",
  2: "lg:grid-cols-2",
  3: "lg:grid-cols-3",
  4: "lg:grid-cols-4",
  5: "lg:grid-cols-5",
  6: "lg:grid-cols-6",
};

const defaultProps = {
  gap: "none" as Gap,
};

export const Grid: Component<GridProps> = (props) => {
  const merged = mergeProps(defaultProps, props);
  const [local, rest] = splitProps(merged, [
    "cols",
    "mdCols",
    "lgCols",
    "gap",
    "children",
    "class",
    "classList",
  ]);

  const classes = () =>
    cn(
      "grid",
      gapMap[local.gap],
      colClassMap[local.cols],
      local.mdCols && mdColClassMap[local.mdCols],
      local.lgCols && lgColClassMap[local.lgCols],
      local.class,
    );

  return (
    <div {...rest} class={classes()} classList={local.classList}>
      {local.children}
    </div>
  );
};
