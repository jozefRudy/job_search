import { ErrorAlert } from "~/components/ui/ErrorAlert";
import { Stack } from "~/components/ui/layout/Stack";
import { DevLayout } from "../DevLayout";

export default function ErrorAlertPage() {
  return (
    <DevLayout title="ErrorAlert" backHref="/dev">
      <Stack gap="md">
        <ErrorAlert>loading jobs: Network Error</ErrorAlert>
        <ErrorAlert>
          loading job: Request failed with status code 500
        </ErrorAlert>
        <ErrorAlert>updating job: Network Error</ErrorAlert>
      </Stack>
    </DevLayout>
  );
}
