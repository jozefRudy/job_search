import { fireEvent, render, screen } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDeleteJob, useGetJob, useListJobs, useRateJob } from "./api";
import { getListJobsQueryKey } from "./generated/orval/jobsearch";

vi.mock("@solidjs/router", () => ({
  useNavigate: () => vi.fn(),
}));

function mockFetch(
  resolver: (url: string) => { body: object; status: number },
) {
  return vi.fn((url: string) => {
    const { body, status } = resolver(url);
    return Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      headers: new Headers(),
      text: () => Promise.resolve(JSON.stringify(body)),
    } as Response);
  });
}

describe("useListJobs", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("refetches when reactive params change", async () => {
    const fetchMock = mockFetch((url) => {
      if (url.includes("page=2")) {
        return {
          body: { jobs: [{ id: 2, title: "Page 2" }], total: 1 },
          status: 200,
        };
      }
      return {
        body: { jobs: [{ id: 1, title: "Page 1" }], total: 1 },
        status: 200,
      };
    });
    global.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    function TestComp() {
      const [page, setPage] = createSignal(1);
      const query = useListJobs(() => ({ page: page(), page_size: 20 }));

      return (
        <div>
          <button type="button" data-testid="next" onClick={() => setPage(2)}>
            Next
          </button>
          <span data-testid="status">
            {query.isPending ? "loading" : "done"}
          </span>
          <span data-testid="first-job">
            {query.data?.jobs[0]?.title ?? "none"}
          </span>
        </div>
      );
    }

    render(() => (
      <QueryClientProvider client={qc}>
        <TestComp />
      </QueryClientProvider>
    ));

    await screen.findByText("done");
    expect(screen.getByTestId("first-job").textContent).toBe("Page 1");

    fireEvent.click(screen.getByTestId("next"));

    await screen.findByText("Page 2", {
      selector: "[data-testid='first-job']",
    });

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toContain("page=1");
    expect(fetchMock.mock.calls[1][0]).toContain("page=2");
  });
});

describe("useGetJob", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("refetches when reactive id changes", async () => {
    const fetchMock = mockFetch((url) => {
      if (url.includes("/jobs/2")) {
        return { body: { id: 2, title: "Job 2" }, status: 200 };
      }
      return { body: { id: 1, title: "Job 1" }, status: 200 };
    });
    global.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    function TestComp() {
      const [id, setId] = createSignal(1);
      const query = useGetJob(id);

      return (
        <div>
          <button type="button" data-testid="switch" onClick={() => setId(2)}>
            Switch
          </button>
          <span data-testid="status">
            {query.isPending ? "loading" : "done"}
          </span>
          <span data-testid="title">{query.data?.title ?? "none"}</span>
        </div>
      );
    }

    render(() => (
      <QueryClientProvider client={qc}>
        <TestComp />
      </QueryClientProvider>
    ));

    await screen.findByText("done");
    expect(screen.getByTestId("title").textContent).toBe("Job 1");

    fireEvent.click(screen.getByTestId("switch"));

    await screen.findByText("Job 2", {
      selector: "[data-testid='title']",
    });

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toContain("/jobs/1");
    expect(fetchMock.mock.calls[1][0]).toContain("/jobs/2");
  });
});

describe("useRateJob", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("invalidates job queries on success", async () => {
    const fetchMock = mockFetch(() => ({
      body: {},
      status: 204,
    }));
    global.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const invalidateSpy = vi.spyOn(qc, "invalidateQueries");

    function TestComp() {
      const mutation = useRateJob();

      return (
        <button
          type="button"
          data-testid="rate"
          onClick={() => mutation.mutate({ id: 1, data: { rating: "liked" } })}
        >
          Rate
        </button>
      );
    }

    render(() => (
      <QueryClientProvider client={qc}>
        <TestComp />
      </QueryClientProvider>
    ));

    fireEvent.click(screen.getByTestId("rate"));

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    await vi.waitFor(() => expect(invalidateSpy).toHaveBeenCalled());

    const listKey = JSON.stringify(getListJobsQueryKey());
    const calls = invalidateSpy.mock.calls.map((c) => JSON.stringify(c[0]));
    expect(calls.some((c) => c.includes(listKey))).toBe(true);
  });
});

describe("useDeleteJob", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("invalidates job queries on success", async () => {
    const fetchMock = mockFetch(() => ({
      body: {},
      status: 204,
    }));
    global.fetch = fetchMock as unknown as typeof globalThis.fetch;

    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const invalidateSpy = vi.spyOn(qc, "invalidateQueries");

    function TestComp() {
      const mutation = useDeleteJob();

      return (
        <button
          type="button"
          data-testid="delete"
          onClick={() => mutation.mutate({ id: 1 })}
        >
          Delete
        </button>
      );
    }

    render(() => (
      <QueryClientProvider client={qc}>
        <TestComp />
      </QueryClientProvider>
    ));

    fireEvent.click(screen.getByTestId("delete"));

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    await vi.waitFor(() => expect(invalidateSpy).toHaveBeenCalled());

    const listKey = JSON.stringify(getListJobsQueryKey());
    const calls = invalidateSpy.mock.calls.map((c) => JSON.stringify(c[0]));
    expect(calls.some((c) => c.includes(listKey))).toBe(true);
  });
});
