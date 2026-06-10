import { createSignal } from "solid-js";
import { Card } from "~/components/ui/Card";
import { ClipboardButton } from "~/components/ui/ClipboardButton";
import { FormField } from "~/components/ui/form/FormField";
import { PasswordInput } from "~/components/ui/form/PasswordInput";

import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { inputStateClasses } from "~/lib/utils";
import { DevLayout } from "../DevLayout";

function MailIcon() {
  return (
    <svg
      class="size-[1em] shrink-0 opacity-50"
      fill="currentColor"
      aria-label="Email"
    >
      <use href="/assets/icons.svg#icon-mail" />
    </svg>
  );
}

function KeyIcon() {
  return (
    <svg
      class="size-[1em] shrink-0 opacity-50"
      fill="currentColor"
      aria-label="Password"
    >
      <use href="/assets/icons.svg#icon-password" />
    </svg>
  );
}

function LinkIcon() {
  return (
    <svg
      class="size-[1em] shrink-0 opacity-50"
      fill="none"
      stroke="currentColor"
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2.5"
      aria-label="URL"
    >
      <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
      <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
    </svg>
  );
}

function validatePassword(value: string): string[] {
  const errors: string[] = [];
  if (value.length < 8) errors.push("Must be at least 8 characters");
  if (!/[A-Z]/.test(value)) errors.push("Must contain an uppercase letter");
  if (/\d/.test(value)) errors.push("Must contain a number");
  return errors;
}

function validateEmail(value: string): string[] {
  const errors: string[] = [];
  if (!value.includes("@")) errors.push("Must contain @");
  if (!value.endsWith(".com") && value.length > 0)
    errors.push("Must end with .com");
  return errors;
}

function validateUrl(value: string): string[] {
  const errors: string[] = [];
  if (value.length === 0) return errors;
  try {
    const u = new URL(value);
    const hostname = u.hostname;
    if (!hostname.includes(".") || hostname.endsWith("."))
      errors.push("Must be valid URL");
  } catch {
    errors.push("Must be valid URL");
  }
  return errors;
}

export default function InputsPage() {
  // --- Interactive section ---
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [url, setUrl] = createSignal("https://");

  const [emailTouched, setEmailTouched] = createSignal(false);
  const [passwordTouched, setPasswordTouched] = createSignal(false);
  const [urlTouched, setUrlTouched] = createSignal(false);

  const [score, setScore] = createSignal(50);
  const [temp, setTemp] = createSignal(0.7);
  const [status, setStatus] = createSignal("");

  const emailErrors = () => validateEmail(email());
  const passwordErrors = () => validatePassword(password());
  const urlErrors = () => validateUrl(url());

  const emailValid = () => email().length > 0 && emailErrors().length === 0;
  const passwordValid = () =>
    password().length > 0 && passwordErrors().length === 0;
  const urlValid = () => url().length > 0 && urlErrors().length === 0;

  return (
    <DevLayout title="Inputs" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Sizes</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <FormField
                  label="Small input"
                  hint="Use size=sm for compact forms"
                >
                  <label class="input input-bordered input-sm w-full">
                    <MailIcon />
                    <input class="grow" placeholder="sm input" />
                  </label>
                </FormField>
                <FormField label="Medium input (default)" hint="Standard size">
                  <label class="input input-bordered w-full">
                    <MailIcon />
                    <input class="grow" placeholder="md input" />
                  </label>
                </FormField>
                <FormField label="Small password" hint="Compact login forms">
                  <PasswordInput
                    size="sm"
                    placeholder="••••••••"
                    startIcon={<KeyIcon />}
                  />
                </FormField>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">FormField</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <FormField label="Email" hint="We will never share your email">
                  <label class="input input-bordered w-full">
                    <MailIcon />
                    <input
                      type="email"
                      class="grow"
                      placeholder="pat@example.com"
                    />
                  </label>
                </FormField>

                <FormField label="Password" hint="Min 8 characters">
                  <PasswordInput
                    placeholder="••••••••"
                    startIcon={<KeyIcon />}
                  />
                </FormField>

                <FormField
                  label="Username"
                  error={["Username already taken", "Must be 3-30 characters"]}
                >
                  <label class="input input-bordered input-error w-full">
                    <svg
                      class="size-[1em] shrink-0 opacity-50"
                      fill="currentColor"
                      aria-label="User"
                    >
                      <use href="/assets/icons.svg#icon-account" />
                    </svg>
                    <input type="text" class="grow" placeholder="username" />
                  </label>
                </FormField>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Interactive</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <FormField
                  label="Email"
                  hint="Type to trigger validation"
                  error={emailTouched() ? emailErrors() : []}
                >
                  <label
                    class="input input-bordered w-full"
                    classList={inputStateClasses({
                      error: emailTouched() && !emailValid(),
                      touched: emailTouched() && emailValid(),
                    })}
                  >
                    <MailIcon />
                    <input
                      type="text"
                      class="grow"
                      placeholder="pat@example.com"
                      value={email()}
                      onInput={(e) => setEmail(e.currentTarget.value)}
                      onBlur={() => setEmailTouched(true)}
                    />
                  </label>
                </FormField>

                <FormField
                  label="Password"
                  hint="Type to trigger validation"
                  error={passwordTouched() ? passwordErrors() : []}
                >
                  <PasswordInput
                    placeholder="••••••••"
                    value={password()}
                    onInput={(e) => setPassword(e.currentTarget.value)}
                    onBlur={() => setPasswordTouched(true)}
                    classList={inputStateClasses({
                      error: passwordTouched() && !passwordValid(),
                      touched: passwordTouched() && passwordValid(),
                    })}
                    startIcon={<KeyIcon />}
                  />
                </FormField>

                <FormField
                  label="Website URL"
                  hint="Must be valid URL"
                  error={urlTouched() ? urlErrors() : []}
                >
                  <label
                    class="input input-bordered w-full"
                    classList={inputStateClasses({
                      error: urlTouched() && !urlValid(),
                      touched: urlTouched() && urlValid(),
                    })}
                  >
                    <LinkIcon />
                    <input
                      type="text"
                      class="grow"
                      placeholder="https://example.com"
                      value={url()}
                      onInput={(e) => setUrl(e.currentTarget.value)}
                      onBlur={() => setUrlTouched(true)}
                    />
                  </label>
                </FormField>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Select</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <FormField
                  label="Page size"
                  hint="Small select for compact forms"
                >
                  <select class="select select-bordered select-sm w-full">
                    <option value="10">10</option>
                    <option value="25">25</option>
                    <option value="50">50</option>
                    <option value="100">100</option>
                  </select>
                </FormField>

                <FormField label="Date range" hint="Standard size">
                  <select class="select select-bordered w-full">
                    <option value="" disabled selected>
                      Choose range
                    </option>
                    <option value="today">Today</option>
                    <option value="week">This week</option>
                    <option value="month">This month</option>
                    <option value="year">This year</option>
                  </select>
                </FormField>

                <FormField
                  label="Filter status"
                  hint="Error state"
                  error={!status() ? ["Selection required"] : []}
                >
                  <select
                    class="select select-bordered w-full"
                    classList={{
                      "select-error": !status(),
                    }}
                    value={status()}
                    onChange={(e) => setStatus(e.currentTarget.value)}
                  >
                    <option value="" disabled>
                      Select status
                    </option>
                    <option value="active">Active</option>
                    <option value="archived">Archived</option>
                  </select>
                </FormField>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Checkbox</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="checkbox checkbox-sm" />
                  <span class="label-text">Small checkbox</span>
                </label>
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="checkbox" />
                  <span class="label-text">Medium checkbox (default)</span>
                </label>
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="checkbox" disabled />
                  <span class="label-text">Disabled</span>
                </label>
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="checkbox" checked />
                  <span class="label-text">Checked</span>
                </label>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Toggle</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="toggle toggle-sm" />
                  <span class="label-text">Small toggle</span>
                </label>
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="toggle" />
                  <span class="label-text">Medium toggle (default)</span>
                </label>
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="toggle" disabled />
                  <span class="label-text">Disabled</span>
                </label>
                <label class="label cursor-pointer justify-start gap-3">
                  <input type="checkbox" class="toggle" checked />
                  <span class="label-text">Checked</span>
                </label>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Range</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <FormField
                  label="Score threshold"
                  hint="Small range inside FormField"
                >
                  <input
                    type="range"
                    class="range range-sm w-full"
                    min={0}
                    max={100}
                    value={score()}
                    onInput={(e) => setScore(Number(e.currentTarget.value))}
                  />
                </FormField>

                <FormField
                  label={`Temperature: ${temp().toFixed(1)}`}
                  hint="Medium range with value above"
                >
                  <Stack gap="sm">
                    <input
                      type="range"
                      class="range w-full"
                      min={0}
                      max={2}
                      step={0.1}
                      value={temp()}
                      onInput={(e) => setTemp(Number(e.currentTarget.value))}
                    />
                  </Stack>
                </FormField>

                <Stack gap="sm">
                  <span class="label-text">With min/max</span>
                  <input
                    type="range"
                    class="range w-full"
                    min={1}
                    max={5}
                    step={1}
                    value={3}
                  />
                  <Row
                    justify="between"
                    class="px-2.5 text-base-content/50 text-xs"
                  >
                    <span>|</span>
                    <span>|</span>
                    <span>|</span>
                    <span>|</span>
                    <span>|</span>
                  </Row>
                  <Row
                    justify="between"
                    class="px-2.5 text-base-content/50 text-xs"
                  >
                    <span>1</span>
                    <span>2</span>
                    <span>3</span>
                    <span>4</span>
                    <span>5</span>
                  </Row>
                </Stack>

                <Stack gap="sm">
                  <span class="label-text">Disabled</span>
                  <input
                    type="range"
                    class="range range-sm w-full"
                    value={30}
                    disabled
                  />
                </Stack>

                <input type="range" class="range w-full" value={70} />
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Native Comparison</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="sm">
                <span class="font-medium text-sm">
                  Datetime Local (native icon)
                </span>
                <input
                  type="datetime-local"
                  class="input input-bordered w-full"
                />
                <p class="text-base-content/50 text-xs">
                  Native browser picker indicator for reference
                </p>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">ClipboardButton</h2>
          <Card class="card-border border-primary bg-base-100">
            <Card.Body>
              <Stack gap="md">
                <Row gap="md" align="center">
                  <ClipboardButton size="sm" text="hello@example.com" />
                  <ClipboardButton size="md" text="hello@example.com" />
                  <ClipboardButton size="lg" text="hello@example.com" />
                </Row>
                <ClipboardButton text="https://api.example.com/v1/posts">
                  Copy API URL
                </ClipboardButton>
              </Stack>
            </Card.Body>
          </Card>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
