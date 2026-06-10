import { Card } from "~/components/ui/Card";
import { Grid } from "~/components/ui/layout/Grid";
import { Row } from "~/components/ui/layout/Row";
import { Stack } from "~/components/ui/layout/Stack";
import { Skeleton } from "~/components/ui/Skeleton";
import { DevLayout } from "../DevLayout";

export default function SkeletonPage() {
  return (
    <DevLayout title="Skeleton" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Block (default)</h2>
          <p class="text-base-content/60 text-sm">
            Standard placeholder. Pass dimensions via class.
          </p>
          <Stack gap="sm">
            <Skeleton class="h-4" />
            <Skeleton class="h-4 w-3/4" />
            <Skeleton class="h-4 w-1/2" />
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Circle</h2>
          <p class="text-base-content/60 text-sm">Avatar placeholders.</p>
          <Row gap="md" align="center">
            <Skeleton variant="circle" class="h-12 w-12" />
            <Skeleton variant="circle" class="h-10 w-10" />
            <Skeleton variant="circle" class="h-8 w-8" />
          </Row>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Text (with content)</h2>
          <p class="text-base-content/60 text-sm">
            Content stays visible under shimmer overlay.
          </p>
          <Skeleton variant="text" class="h-6">
            AI is thinking harder...
          </Skeleton>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Card Skeleton</h2>
          <Grid cols={1} mdCols={2} gap="md">
            <Card>
              <Card.Body>
                <Stack gap="md">
                  <Skeleton class="h-6 w-2/3" />
                  <Skeleton class="h-4" />
                  <Skeleton class="h-4 w-5/6" />
                  <Row gap="sm">
                    <Skeleton class="h-8 w-20" />
                    <Skeleton class="h-8 w-20" />
                  </Row>
                </Stack>
              </Card.Body>
            </Card>
            <Card>
              <Card.Body>
                <Stack gap="md">
                  <Row gap="md" align="center">
                    <Skeleton variant="circle" class="h-12 w-12" />
                    <Stack gap="sm" class="flex-1">
                      <Skeleton class="h-5 w-32" />
                      <Skeleton class="h-4" />
                    </Stack>
                  </Row>
                  <Skeleton class="h-4" />
                  <Skeleton class="h-4 w-4/5" />
                </Stack>
              </Card.Body>
            </Card>
          </Grid>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
