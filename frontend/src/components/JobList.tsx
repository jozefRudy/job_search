import { useNavigate, useSearchParams } from "@solidjs/router";
import { isNotNil, pickBy } from "es-toolkit";
import { createMemo, Show } from "solid-js";
import { z } from "zod";
import {
  type Job,
  type ListJobsParams,
  type Platform,
  type Rating,
  type Sort,
  useListJobs,
  useRateJob,
} from "~/api";
import { Button } from "~/components/ui/Button";
import { Pagination } from "~/components/ui/data/Pagination";
import { Table } from "~/components/ui/data/Table";
import { Container } from "~/components/ui/layout/Container";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { Skeleton } from "~/components/ui/Skeleton";
import { cn, fmtRelative, ratingClass, ratingEmoji } from "~/lib/utils";

const PAGE_SIZE = 20;

const PLATFORM_SORTS: Record<
  Platform | "all",
  ReadonlyArray<{ value: Sort; label: string }>
> = {
  all: [{ value: "created", label: "Created" }],
  upwork: [
    { value: "created", label: "Created" },
    { value: "upwork_viewed", label: "Last viewed" },
  ],
  nofluffjobs: [{ value: "created", label: "Created" }],
  efinancialcareers: [{ value: "created", label: "Created" }],
};

export function JobList() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const platform = (): Platform | null =>
    z
      .union([
        z.literal("upwork"),
        z.literal("nofluffjobs"),
        z.literal("efinancialcareers"),
      ])
      .nullable()
      .catch(null)
      .parse(searchParams.platform);

  const ratingFilter = (): Rating | null =>
    z
      .union([z.literal("liked"), z.literal("neutral"), z.literal("disliked")])
      .nullable()
      .catch(null)
      .parse(searchParams.rating);

  const sortBy = (): Sort => {
    const supported = PLATFORM_SORTS[platform() ?? "all"].map((s) => s.value);
    const schema = z.enum(supported as [Sort, ...Sort[]]).catch(supported[0]);
    return schema.parse(searchParams.sort_by);
  };

  const page = (): number =>
    z.coerce.number().int().positive().catch(1).parse(searchParams.page);

  const params = (): ListJobsParams => {
    return pickBy(
      {
        sort_by: sortBy(),
        page: page(),
        page_size: PAGE_SIZE,
        platform: platform(),
        rating: ratingFilter(),
      },
      isNotNil,
    ) as ListJobsParams;
  };

  const query = useListJobs(params);
  const rateMutation = useRateJob();

  const jobs = () => query.data?.jobs ?? [];
  const total = () => query.data?.total ?? 0;

  const hasUpwork = createMemo(() =>
    jobs().some((j) => j.platform === "upwork"),
  );

  function setPlatformAndReset(p: Platform | null) {
    const supported = new Set(PLATFORM_SORTS[p ?? "all"].map((s) => s.value));
    const nextSort = supported.has(sortBy()) ? sortBy() : "created";
    setSearchParams(
      { platform: p, sort_by: nextSort, page: "" },
      { replace: true },
    );
  }

  function setRatingAndReset(r: Rating | null) {
    setSearchParams({ rating: r, page: "" }, { replace: true });
  }

  function setSortByAndReset(s: Sort) {
    setSearchParams({ sort_by: s, page: "" }, { replace: true });
  }

  function setPageAndUpdate(p: number) {
    setSearchParams({ page: p === 1 ? "" : String(p) }, { replace: true });
  }

  function handleRate(job: Job, rating: Rating) {
    if (job.id == null) return;
    rateMutation.mutate({ id: job.id, data: { rating } });
  }

  const columns = () => {
    const base = [
      {
        key: "id",
        header: "Id",
        cell: (j: Job) => (
          <button
            type="button"
            class="link link-primary"
            onClick={() => j.id != null && navigate(`/jobs/${j.id}`)}
          >
            {j.id}
          </button>
        ),
      },
      ...(platform() == null
        ? [
            {
              key: "platform",
              header: "Platform",
              accessor: (j: Job) => j.platform,
            },
          ]
        : []),
      {
        key: "posted",
        header: "Posted",
        accessor: (j: Job) => fmtRelative(j.created_at),
      },
      {
        key: "budget",
        header: "Budget",
        accessor: (j: Job) => j.budget ?? "?",
      },
      {
        key: "applied",
        header: "Applied",
        accessor: (j: Job) => fmtRelative(j.applied_at),
      },
      {
        key: "rating",
        header: "Rating",
        cell: (j: Job) => (
          <span class={cn(ratingClass(j.liked), "font-bold")}>
            {ratingEmoji(j.liked)}
          </span>
        ),
      },
      {
        key: "actions",
        header: "",
        class: "min-w-[100px]",
        cell: (j: Job) => (
          <Row gap="sm" align="center">
            <Button
              variant="ghost"
              size="sm"
              class={cn(j.liked === true && "text-success")}
              onClick={() => handleRate(j, "liked")}
            >
              👍
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class={cn(j.liked === false && "text-error")}
              onClick={() => handleRate(j, "disliked")}
            >
              👎
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class={cn(j.liked === null && "text-warning")}
              onClick={() => handleRate(j, "neutral")}
            >
              ↔️
            </Button>
          </Row>
        ),
      },
      ...(hasUpwork()
        ? [
            {
              key: "last_viewed",
              header: "Last viewed",
              accessor: (j: Job) =>
                j.raw.platform === "upwork"
                  ? fmtRelative(j.raw.detail.last_viewed)
                  : "",
            },
          ]
        : []),
      { key: "title", header: "Title", accessor: (j: Job) => j.title },
    ];
    return base;
  };

  return (
    <Container maxWidth="lg" paddingX="sm" class="py-6">
      <Stack gap="md">
        <h1 class="font-bold text-2xl">Jobs</h1>

        <Row gap="md" align="center" class="flex-wrap">
          <select
            class="select select-sm"
            value={platform() ?? "all"}
            onChange={(e) =>
              setPlatformAndReset(
                e.currentTarget.value === "all"
                  ? null
                  : (e.currentTarget.value as Platform),
              )
            }
          >
            <option value="all">All platforms</option>
            <option value="upwork">Upwork</option>
            <option value="nofluffjobs">NoFluffJobs</option>
            <option value="efinancialcareers">eFinancialCareers</option>
          </select>

          <select
            class="select select-sm"
            value={ratingFilter() ?? "all"}
            onChange={(e) =>
              setRatingAndReset(
                e.currentTarget.value === "all"
                  ? null
                  : (e.currentTarget.value as Rating),
              )
            }
          >
            <option value="all">No filter</option>
            <option value="liked">Liked</option>
            <option value="neutral">Neutral</option>
            <option value="disliked">Disliked</option>
          </select>

          <Show when={PLATFORM_SORTS[platform() ?? "all"].length > 1}>
            <select
              class="select select-sm"
              value={sortBy()}
              onChange={(e) => setSortByAndReset(e.currentTarget.value as Sort)}
            >
              {PLATFORM_SORTS[platform() ?? "all"].map((s) => (
                <option value={s.value}>{s.label}</option>
              ))}
            </select>
          </Show>
        </Row>

        <Show when={!query.isPending} fallback={<Skeleton class="h-64" />}>
          <Show
            when={!query.error}
            fallback={
              <div class="text-error">Error: {query.error?.message}</div>
            }
          >
            <Table
              columns={columns()}
              data={jobs()}
              zebra
              hoverable
              emptyMessage="No jobs match the current filter"
            />
            <Pagination
              currentPage={page()}
              totalItems={total()}
              pageSize={PAGE_SIZE}
              onPageChange={setPageAndUpdate}
            />
          </Show>
        </Show>
      </Stack>
    </Container>
  );
}
