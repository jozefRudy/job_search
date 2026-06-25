import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { match } from "ts-pattern";

export function cn(...inputs: (string | undefined | null | false)[]) {
  return twMerge(clsx(inputs));
}

export function formatNumber(value: number, decimals = 0): string {
  return value.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

export function inputStateClasses(props: {
  error?: boolean;
  touched?: boolean;
}): Record<string, boolean> {
  return {
    "input-error": !!props.error,
    "input-success": !!props.touched && !props.error,
  };
}

export function fmtRelative(dtStr: string | null | undefined): string {
  if (!dtStr) return "";
  const dt = new Date(dtStr);
  const now = new Date();
  const mins = Math.floor((now.getTime() - dt.getTime()) / 60000);
  const hrs = Math.floor(mins / 60);
  const days = Math.floor(hrs / 24);

  return match({ mins, hrs, days })
    .when(
      ({ mins }) => mins < 1,
      () => "just now",
    )
    .when(
      ({ mins }) => mins < 60,
      ({ mins }) => `${mins}m ago`,
    )
    .when(
      ({ hrs }) => hrs < 24,
      ({ hrs }) => `${hrs}h ago`,
    )
    .when(
      ({ days }) => days < 7,
      ({ days }) => `${days}d ago`,
    )
    .otherwise(({ days }) => `${Math.floor(days / 7)}w ago`);
}

export function ratingEmoji(rating: string | null | undefined): string {
  return match(rating)
    .with("liked", () => "👍")
    .with("disliked", () => "👎")
    .otherwise(() => "↔️");
}

export function ratingClass(rating: string | null | undefined): string {
  return match(rating)
    .with("liked", () => "text-success")
    .with("disliked", () => "text-error")
    .otherwise(() => "text-base-content/40");
}

export function ellip(value: string | null | undefined, max: number): string {
  const s = value ?? "";
  if (s.length <= max) return s;
  return `${s.slice(0, max)}…`;
}
