import { createSignal } from "solid-js";
import { Pagination } from "~/components/ui/data/Pagination";
import { Stack } from "~/components/ui/layout/Stack";
import { DevLayout } from "../DevLayout";

export default function PaginationPage() {
  const [page1, setPage1] = createSignal(1);
  const [page2, setPage2] = createSignal(5);
  const [page3, setPage3] = createSignal(100);

  return (
    <DevLayout title="Pagination Kitchen Sink" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Few Pages</h2>
          <Pagination
            currentPage={page1()}
            totalItems={187}
            pageSize={20}
            capped={false}
            onPageChange={setPage1}
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Many Pages</h2>
          <Pagination
            currentPage={page2()}
            totalItems={1984}
            pageSize={20}
            capped={false}
            onPageChange={setPage2}
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Last Page</h2>
          <Pagination
            currentPage={page3()}
            totalItems={1984}
            pageSize={20}
            capped={false}
            onPageChange={setPage3}
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Single Page</h2>
          <Pagination
            currentPage={1}
            totalItems={5}
            pageSize={20}
            capped={false}
            onPageChange={() => {}}
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">0 Items</h2>
          <p class="text-base-content/60 text-sm">
            Paginator hidden when no items.
          </p>
          <Pagination
            currentPage={1}
            totalItems={0}
            pageSize={20}
            capped={false}
            onPageChange={() => {}}
          />
        </Stack>
      </Stack>
    </DevLayout>
  );
}
