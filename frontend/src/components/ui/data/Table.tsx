import type { JSX } from "solid-js";
import { For, mergeProps, Show } from "solid-js";
import { ErrorAlert } from "~/components/ui/ErrorAlert";
import { Skeleton } from "~/components/ui/Skeleton";
import { cn } from "~/lib/utils";

export type TableSize = "sm" | "md" | "lg";
export type TableLoadState = "normal" | "pending" | "fetching" | "error";

export interface ColumnDef<T> {
  key: string;
  header: string;
  accessor?: (row: T) => unknown;
  format?: (value: unknown, row: T) => string;
  cell?: (row: T) => JSX.Element;
  class?: string;
}

export interface TableProps<T> {
  columns: ColumnDef<T>[];
  data: T[];
  size?: TableSize;
  zebra?: boolean;
  hoverable?: boolean;
  pinRows?: boolean;
  pinCols?: boolean;
  loadState?: TableLoadState;
  emptyMessage?: string;
  error?: JSX.Element;
  class?: string;
}

const sizeMap: Record<TableSize, string> = {
  sm: "table-sm",
  md: "",
  lg: "table-lg",
};

const defaults = {
  size: "md" as TableSize,
  zebra: false,
  hoverable: false,
  pinRows: false,
  pinCols: false,
  loadState: "normal" as TableLoadState,
  emptyMessage: "No data",
};

export function Table<T>(props: TableProps<T>) {
  const merged = mergeProps(defaults, props);

  const classes = cn(
    "table",
    sizeMap[merged.size],
    merged.zebra && "table-zebra",
    merged.pinRows && "table-pin-rows",
    merged.pinCols && "table-pin-cols",
    merged.class,
  );

  const colCount = () => merged.columns.length;

  function renderCell(col: ColumnDef<T>, row: T): JSX.Element {
    if (col.cell) {
      return col.cell(row);
    }

    const value = col.accessor
      ? col.accessor(row)
      : (row as Record<string, unknown>)[col.key];

    const text = col.format ? col.format(value, row) : String(value ?? "");

    return <span>{text}</span>;
  }

  return (
    <div class="relative w-full overflow-x-auto bg-base-100">
      <Show when={merged.loadState === "fetching"}>
        <div class="absolute top-3 right-3 z-40">
          <span class="loading loading-spinner loading-sm" />
        </div>
      </Show>

      <table
        class={cn(classes, merged.loadState === "fetching" && "opacity-60")}
      >
        <thead>
          <tr>
            <For each={merged.columns}>
              {(col) => <th class={cn(col.class)}>{col.header}</th>}
            </For>
          </tr>
        </thead>
        <tbody>
          <Show when={merged.loadState === "error"}>
            <tr>
              <td colSpan={colCount()} class="py-12 text-center">
                <ErrorAlert>
                  {merged.error ?? "Something went wrong"}
                </ErrorAlert>
              </td>
            </tr>
          </Show>

          <Show when={merged.loadState !== "error"}>
            <Show
              when={merged.data.length > 0 || merged.loadState === "pending"}
              fallback={
                <tr>
                  <td colSpan={colCount()} class="py-12 text-center">
                    <div class="flex flex-col items-center gap-3 text-base-content/40">
                      <svg
                        class="h-8 w-8"
                        fill="currentColor"
                        aria-label="No data"
                      >
                        <use href="/assets/icons.svg#icon-info" />
                      </svg>
                      <span class="text-sm">{merged.emptyMessage}</span>
                    </div>
                  </td>
                </tr>
              }
            >
              <Show
                when={merged.loadState === "pending"}
                fallback={
                  <For each={merged.data}>
                    {(row) => (
                      <tr class={merged.hoverable ? "row-hover" : undefined}>
                        <For each={merged.columns}>
                          {(col) => (
                            <td class={cn("text-nowrap", col.class)}>
                              {renderCell(col, row)}
                            </td>
                          )}
                        </For>
                      </tr>
                    )}
                  </For>
                }
              >
                {Array.from({ length: 5 }).map(() => (
                  <tr>
                    <For each={merged.columns}>
                      {() => (
                        <td>
                          <Skeleton class="h-4" />
                        </td>
                      )}
                    </For>
                  </tr>
                ))}
              </Show>
            </Show>
          </Show>
        </tbody>
      </table>
    </div>
  );
}
