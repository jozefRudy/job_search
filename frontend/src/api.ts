import { useNavigate } from "@solidjs/router";
import { createQuery, useQueryClient } from "@tanstack/solid-query";
import {
  applyJob,
  deleteJob,
  getGetJobQueryKey,
  getJob,
  getListJobsQueryKey,
  listJobs,
  rateJob,
  useApplyJob as useApplyJobGen,
  useDeleteJob as useDeleteJobGen,
  useRateJob as useRateJobGen,
} from "~/generated/orval/jobsearch";
import type { ListJobsParams } from "~/generated/orval/jobsearch.schemas";

export type {
  ApplyRequest,
  Job,
  ListJobsParams,
  Platform,
  RateRequest,
  Rating,
  Sort,
} from "~/generated/orval/jobsearch.schemas";

async function unwrap<T>(
  promise: Promise<{ data: T; status: number } | { status: number }>,
  message: string,
): Promise<T> {
  const res = await promise;
  if (res.status < 200 || res.status >= 300) {
    throw new Error(message);
  }
  return (res as { data: T; status: number }).data;
}

async function throwOnError<T extends { status: number }>(
  promise: Promise<T>,
  message: string,
): Promise<T> {
  const res = await promise;
  if (res.status < 200 || res.status >= 300) {
    throw new Error(message);
  }
  return res;
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
  return useRateJobGen<Error>({
    mutation: {
      mutationFn: (variables) =>
        throwOnError(
          rateJob(variables.id, variables.data),
          "Failed to update rating",
        ),
      onSuccess: (_data, variables) => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        qc.invalidateQueries({ queryKey: getGetJobQueryKey(variables.id) });
      },
    },
  });
}

export function useApplyJob() {
  const qc = useQueryClient();
  return useApplyJobGen<Error>({
    mutation: {
      mutationFn: (variables) =>
        throwOnError(
          applyJob(variables.id, variables.data),
          "Failed to update application",
        ),
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
  return useDeleteJobGen<Error>({
    mutation: {
      mutationFn: (variables) =>
        throwOnError(deleteJob(variables.id), "Failed to delete job"),
      onSuccess: () => {
        qc.invalidateQueries({ queryKey: getListJobsQueryKey() });
        navigate(-1);
      },
    },
  });
}
