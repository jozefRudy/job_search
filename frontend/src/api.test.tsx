import { fireEvent, render, screen } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import axios from "axios";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDeleteJob, useGetJob, useListJobs, useRateJob } from "./api";
import { getListJobsQueryKey } from "./generated/orval/jobsearch";

vi.mock("@solidjs/router", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    delete: vi.fn(),
  },
}));

function mockAxiosResponse<T>(data: T, status = 200) {
  return Promise.resolve({
    data,
    status,
    statusText: status === 204 ? "No Content" : "OK",
    headers: {},
    config: {},
  });
}

describe("useListJobs", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("refetches when reactive params change", async () => {
    const axiosMock = vi.mocked(axios);
    axiosMock.get.mockImplementation(
      (_url: string, config?: { params?: { page?: number } }) => {
        const page = config?.params?.page ?? 1;
        return mockAxiosResponse({
          jobs: [{ id: page, title: `Page ${page}` }],
          total: 1,
        });
      },
    );

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

    expect(axiosMock.get).toHaveBeenCalledTimes(2);
    expect(axiosMock.get.mock.calls[0][1]?.params).toMatchObject({
      page: 1,
      page_size: 20,
    });
    expect(axiosMock.get.mock.calls[1][1]?.params).toMatchObject({
      page: 2,
      page_size: 20,
    });
  });
});

describe("useGetJob", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("refetches when reactive id changes", async () => {
    const axiosMock = vi.mocked(axios);
    axiosMock.get.mockImplementation((url: string) => {
      const id = url.endsWith("/2") ? 2 : 1;
      return mockAxiosResponse({ id, title: `Job ${id}` });
    });

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

    const jobUrls = axiosMock.get.mock.calls
      .map((call) => String(call[0]))
      .filter((url) => url.includes("/jobs/"));
    expect(jobUrls).toEqual(
      expect.arrayContaining(["/api/jobs/1", "/api/jobs/2"]),
    );
  });
});

describe("useRateJob", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("exposes error on failure", async () => {
    const axiosMock = vi.mocked(axios);
    axiosMock.post.mockRejectedValue(new Error("Network error"));

    const qc = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    function TestComp() {
      const mutation = useRateJob();

      return (
        <>
          <button
            type="button"
            data-testid="rate"
            onClick={() =>
              mutation.mutate({ id: 1, data: { rating: "liked" } })
            }
          >
            Rate
          </button>
          <span data-testid="error">
            {mutation.error?.message ?? "no error"}
          </span>
        </>
      );
    }

    render(() => (
      <QueryClientProvider client={qc}>
        <TestComp />
      </QueryClientProvider>
    ));

    fireEvent.click(screen.getByTestId("rate"));

    await screen.findByText("Network error", {
      selector: "[data-testid='error']",
    });
  });

  it("invalidates job queries on success", async () => {
    const axiosMock = vi.mocked(axios);
    axiosMock.post.mockImplementation(() => mockAxiosResponse({}, 204));

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

    await vi.waitFor(() => expect(axiosMock.post).toHaveBeenCalledTimes(1));
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

  it("exposes error on failure", async () => {
    const axiosMock = vi.mocked(axios);
    axiosMock.delete.mockRejectedValue(new Error("Delete failed"));

    const qc = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    function TestComp() {
      const mutation = useDeleteJob();

      return (
        <>
          <button
            type="button"
            data-testid="delete"
            onClick={() => mutation.mutate({ id: 1 })}
          >
            Delete
          </button>
          <span data-testid="error">
            {mutation.error?.message ?? "no error"}
          </span>
        </>
      );
    }

    render(() => (
      <QueryClientProvider client={qc}>
        <TestComp />
      </QueryClientProvider>
    ));

    fireEvent.click(screen.getByTestId("delete"));

    await screen.findByText("Delete failed", {
      selector: "[data-testid='error']",
    });
  });

  it("invalidates job queries on success", async () => {
    const axiosMock = vi.mocked(axios);
    axiosMock.delete.mockImplementation(() => mockAxiosResponse({}, 204));

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

    await vi.waitFor(() => expect(axiosMock.delete).toHaveBeenCalledTimes(1));
    await vi.waitFor(() => expect(invalidateSpy).toHaveBeenCalled());

    const listKey = JSON.stringify(getListJobsQueryKey());
    const calls = invalidateSpy.mock.calls.map((c) => JSON.stringify(c[0]));
    expect(calls.some((c) => c.includes(listKey))).toBe(true);
  });
});
