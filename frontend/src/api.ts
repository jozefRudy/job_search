import { useNavigate } from "@solidjs/router";
import { createQuery, useQueryClient } from "@tanstack/solid-query";
import {
  getGetJobQueryKey,
  getJob,
  getListJobsQueryKey,
  listJobs,
  useDeleteJob as useDeleteJobGen,
  useRateJob as useRateJobGen,
} from "~/generated/orval/jobsearch";
import type { ListJobsParams } from "~/generated/orval/jobsearch.schemas";

export type {
  Job,
  ListJobsParams,
  Platform,
  Rating,
  Sort,
} from "~/generated/orval/jobsearch.schemas";

async function unwrap<T>(
  promise: Promise<{ data: T; status: 200 } | { status: number }>,
  message: string,
): Promise<T> {
  const res = await promise;
  if (res.status !== 200) throw new Error(message);
  return (res as { data: T; status: 200 }).data;
}

export function useListJobs(params: () => ListJobsParams) {
  return createQuery(() => ({
    queryKey: getListJobsQueryKey(params()),
    queryFn: () => unwrap(listJobs(params()), "Failed to fetch jobs"),
    structuralSharing: false,
  }));
}

export function useGetJob(id: () => number) {
  return createQuery(() => ({
    queryKey: getGetJobQueryKey(id()),
    queryFn: () => unwrap(getJob(id()), "Failed to fetch job"),
    enabled: id() != null && !Number.isNaN(id()),
    structuralSharing: false,
  }));
}

export function useRateJob() {
  const qc = useQueryClient();
  return useRateJobGen({
    mutation: {
      onSuccess: (_data, variables) => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        qc.invalidateQueries({ queryKey: getGetJobQueryKey(variables.id) });
      },
    },
  });
}

export function useDeleteJob() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  return useDeleteJobGen({
    mutation: {
      onSuccess: () => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        navigate(-1);
      },
    },
  });
}
