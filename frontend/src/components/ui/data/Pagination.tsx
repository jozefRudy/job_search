import { createEffect, For, Show } from "solid-js";
import { match } from "ts-pattern";
import { cn } from "~/lib/utils";

export interface PaginationProps {
  currentPage: number;
  totalItems: number;
  pageSize: number;
  capped: boolean;
  onPageChange: (page: number) => void;
}

function formatRange(
  currentPage: number,
  totalItems: number,
  pageSize: number,
  capped: boolean,
): string {
  const start = (currentPage - 1) * pageSize + 1;
  const end = Math.min(currentPage * pageSize, totalItems);
  const totalPages = Math.ceil(totalItems / pageSize);
  const totalSuffix = capped ? `+` : "";

  return match({ totalItems, totalPages })
    .when(
      ({ totalItems }) => totalItems === 0,
      () => "0 items",
    )
    .when(
      ({ totalItems }) => totalItems === 1,
      () => "1 item",
    )
    .when(
      ({ totalPages }) => totalPages <= 1,
      ({ totalItems }) => `${totalItems}${totalSuffix} items`,
    )
    .otherwise(() => `${start}-${end} of ${totalItems}${totalSuffix}`);
}

export function Pagination(props: PaginationProps) {
  const totalPages = () => Math.ceil(props.totalItems / props.pageSize);

  createEffect(() => {
    const tp = totalPages();
    if (tp > 0 && props.currentPage > tp) {
      props.onPageChange(tp);
    }
  });

  const rangeText = () =>
    formatRange(
      props.currentPage,
      props.totalItems,
      props.pageSize,
      props.capped,
    );

  const pages = () => {
    const tp = totalPages();
    return match({ currentPage: props.currentPage, totalPages: tp })
      .when(
        ({ totalPages }) => totalPages <= 7,
        ({ totalPages }) => Array.from({ length: totalPages }, (_, i) => i + 1),
      )
      .when(
        ({ currentPage }) => currentPage <= 4,
        ({ totalPages }) => [1, 2, 3, 4, 5, "ellipsis", totalPages],
      )
      .when(
        ({ currentPage, totalPages }) => currentPage >= totalPages - 3,
        ({ totalPages }) => [
          1,
          "ellipsis",
          totalPages - 4,
          totalPages - 3,
          totalPages - 2,
          totalPages - 1,
          totalPages,
        ],
      )
      .otherwise(({ currentPage, totalPages }) => [
        1,
        "ellipsis",
        currentPage - 1,
        currentPage,
        currentPage + 1,
        "ellipsis",
        totalPages,
      ]) as (number | "ellipsis")[];
  };

  return (
    <Show when={props.totalItems > 0}>
      <div class="flex items-center gap-4">
        <span class="text-base-content/60 text-sm tabular-nums">
          {rangeText()}
        </span>
        <Show when={totalPages() > 1}>
          <div class="join">
            <button
              type="button"
              class="join-item btn"
              disabled={props.currentPage === 1}
              onClick={() => props.onPageChange(props.currentPage - 1)}
            >
              «
            </button>
            <For each={pages()}>
              {(page) => (
                <Show
                  when={page !== "ellipsis"}
                  fallback={
                    <button type="button" class="join-item btn btn-disabled">
                      ...
                    </button>
                  }
                >
                  <button
                    type="button"
                    class={cn(
                      "join-item btn",
                      page === props.currentPage && "btn-primary",
                    )}
                    aria-current={
                      page === props.currentPage ? "page" : undefined
                    }
                    onClick={() => props.onPageChange(page as number)}
                  >
                    {page}
                  </button>
                </Show>
              )}
            </For>
            <button
              type="button"
              class="join-item btn"
              disabled={props.currentPage === totalPages()}
              onClick={() => props.onPageChange(props.currentPage + 1)}
            >
              »
            </button>
          </div>
        </Show>
      </div>
    </Show>
  );
}
