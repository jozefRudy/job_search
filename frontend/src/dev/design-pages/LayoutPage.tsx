import { Button } from "~/components/ui/Button";
import { Card } from "~/components/ui/Card";
import { Container } from "~/components/ui/layout/Container";
import { Grid } from "~/components/ui/layout/Grid";
import { Row } from "~/components/ui/layout/Row";
import { Section } from "~/components/ui/layout/Section";
import { Stack } from "~/components/ui/layout/Stack";
import { DevLayout } from "../DevLayout";

function Placeholder({ label }: { label: string }) {
  return (
    <div class="flex items-center justify-center rounded-lg bg-base-300 p-4 font-medium text-sm">
      {label}
    </div>
  );
}

export default function LayoutPage() {
  return (
    <DevLayout title="Layout Primitives" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Stack (vertical)</h2>
          <Card class="card-border border-primary">
            <Card.Body>
              <Stack gap="md">
                <Placeholder label="Item 1" />
                <Placeholder label="Item 2" />
                <Placeholder label="Item 3" />
              </Stack>
            </Card.Body>
          </Card>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Gap</span>
            <Grid cols={2} mdCols={4} gap="md">
              {(["none", "sm", "md", "lg"] as const).map((g) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">{g}</span>
                      <Stack gap={g} class="rounded-lg bg-base-300 p-2">
                        <div class="h-3 rounded bg-primary" />
                        <div class="h-3 rounded bg-primary" />
                      </Stack>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Align</span>
            <Grid cols={2} mdCols={4} gap="md">
              {(["start", "center", "end", "stretch"] as const).map((a) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">
                        align="{a}"
                      </span>
                      <Stack
                        align={a}
                        gap="sm"
                        class="h-24 rounded-lg bg-base-300 p-2"
                      >
                        <div
                          class={`h-4 rounded bg-primary ${a === "stretch" ? "w-full" : "w-12"}`}
                        />
                        <div
                          class={`h-4 rounded bg-primary ${a === "stretch" ? "w-full" : "w-20"}`}
                        />
                      </Stack>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Justify</span>
            <Grid cols={2} mdCols={3} gap="md">
              {(
                [
                  "start",
                  "center",
                  "end",
                  "between",
                  "around",
                  "evenly",
                ] as const
              ).map((j) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">
                        justify="{j}"
                      </span>
                      <Stack
                        justify={j}
                        gap="sm"
                        class="h-32 rounded-lg bg-base-300 p-2"
                      >
                        <div class="h-6 rounded bg-primary" />
                        <div class="h-6 rounded bg-primary" />
                      </Stack>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Row (horizontal)</h2>
          <Card class="card-border border-primary">
            <Card.Body>
              <Row gap="md">
                <Placeholder label="A" />
                <Placeholder label="B" />
                <Placeholder label="C" />
              </Row>
            </Card.Body>
          </Card>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Gap</span>
            <Grid cols={2} mdCols={4} gap="md">
              {(["none", "sm", "md", "lg"] as const).map((g) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">{g}</span>
                      <Row gap={g} class="rounded-lg bg-base-300 p-2">
                        <div class="h-6 w-6 rounded bg-primary" />
                        <div class="h-6 w-6 rounded bg-primary" />
                      </Row>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Align</span>
            <Grid cols={2} mdCols={4} gap="md">
              {(["start", "center", "end", "stretch"] as const).map((a) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">
                        align="{a}"
                      </span>
                      <Row
                        align={a}
                        gap="sm"
                        class="h-24 rounded-lg bg-base-300 p-2"
                      >
                        <div
                          class={`w-6 rounded bg-primary ${a === "stretch" ? "h-full" : "h-4"}`}
                        />
                        <div
                          class={`w-6 rounded bg-primary ${a === "stretch" ? "h-full" : "h-8"}`}
                        />
                      </Row>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Justify</span>
            <Grid cols={2} mdCols={3} gap="md">
              {(
                [
                  "start",
                  "center",
                  "end",
                  "between",
                  "around",
                  "evenly",
                ] as const
              ).map((j) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">
                        justify="{j}"
                      </span>
                      <Row
                        justify={j}
                        gap="sm"
                        class="rounded-lg bg-base-300 p-2"
                      >
                        <div class="h-6 w-6 rounded bg-primary" />
                        <div class="h-6 w-6 rounded bg-primary" />
                      </Row>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Grid</h2>
          <Card class="card-border border-primary">
            <Card.Body>
              <Grid cols={3} gap="md">
                <Placeholder label="1" />
                <Placeholder label="2" />
                <Placeholder label="3" />
                <Placeholder label="4" />
                <Placeholder label="5" />
                <Placeholder label="6" />
              </Grid>
            </Card.Body>
          </Card>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">
              Responsive: cols=1 mdCols=2 lgCols=3
            </span>
            <Grid cols={1} mdCols={3} gap="md">
              <Card class="card-border border-primary">
                <Card.Body>
                  <Stack gap="sm">
                    <span class="text-base-content/60 text-xs">
                      Mobile (&lt; 768px)
                    </span>
                    <Grid cols={1} gap="sm" class="rounded-lg bg-base-300 p-2">
                      <div class="h-8 rounded bg-primary" />
                      <div class="h-8 rounded bg-primary" />
                      <div class="h-8 rounded bg-primary" />
                    </Grid>
                  </Stack>
                </Card.Body>
              </Card>
              <Card class="card-border border-primary">
                <Card.Body>
                  <Stack gap="sm">
                    <span class="text-base-content/60 text-xs">
                      Tablet (768px+)
                    </span>
                    <Grid cols={2} gap="sm" class="rounded-lg bg-base-300 p-2">
                      <div class="h-8 rounded bg-primary" />
                      <div class="h-8 rounded bg-primary" />
                      <div class="h-8 rounded bg-primary" />
                    </Grid>
                  </Stack>
                </Card.Body>
              </Card>
              <Card class="card-border border-primary">
                <Card.Body>
                  <Stack gap="sm">
                    <span class="text-base-content/60 text-xs">
                      Desktop (1024px+)
                    </span>
                    <Grid cols={3} gap="sm" class="rounded-lg bg-base-300 p-2">
                      <div class="h-8 rounded bg-primary" />
                      <div class="h-8 rounded bg-primary" />
                      <div class="h-8 rounded bg-primary" />
                    </Grid>
                  </Stack>
                </Card.Body>
              </Card>
            </Grid>
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Container</h2>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Max Width</span>
            <Stack gap="md">
              {(["none", "sm", "md", "lg"] as const).map((mw) => (
                <Container
                  maxWidth={mw}
                  paddingX="sm"
                  class="rounded-lg border border-primary bg-base-200"
                >
                  <span class="text-sm">maxWidth="{mw}"</span>
                </Container>
              ))}
            </Stack>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Padding</span>
            <Stack gap="sm">
              {(["none", "sm", "md", "lg"] as const).map((p) => (
                <Stack gap="sm">
                  <span class="text-base-content/60 text-xs">
                    paddingX="{p}"
                  </span>
                  <Container
                    maxWidth="none"
                    paddingX={p}
                    class="rounded-lg border border-primary bg-base-200"
                  >
                    <div class="h-6 rounded-sm bg-primary" />
                  </Container>
                </Stack>
              ))}
            </Stack>
          </Stack>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">Section</h2>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">Padding</span>
            <Stack gap="sm">
              {(["none", "sm", "md", "lg"] as const).map((p) => (
                <Stack gap="sm">
                  <span class="text-base-content/60 text-xs">
                    paddingY="{p}"
                  </span>
                  <Section
                    bg="base-200"
                    paddingY={p}
                    class="border border-primary"
                  >
                    <div class="h-6 rounded-sm bg-primary" />
                  </Section>
                </Stack>
              ))}
            </Stack>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">
              Background × Padding
            </span>
            <Grid cols={2} mdCols={3} gap="md">
              {(
                [
                  "base-100",
                  "base-200",
                  "base-300",
                  "primary",
                  "accent",
                ] as const
              ).map((b) => (
                <Card class="card-border border-primary">
                  <Card.Body>
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">
                        bg="{b}" paddingY="md"
                      </span>
                      <Section bg={b} paddingY="md">
                        <div class="h-6 rounded-sm bg-primary" />
                      </Section>
                    </Stack>
                  </Card.Body>
                </Card>
              ))}
            </Grid>
          </Stack>

          <Stack gap="sm">
            <span class="text-base-content/60 text-sm">
              Pattern: full-bleed background + contained content
            </span>
            <Card class="card-border border-primary">
              <Card.Body>
                <Stack gap="sm">
                  <span class="text-base-content/60 text-xs">
                    Section bg="base-200" paddingY="lg"
                  </span>
                  <Section bg="base-200" paddingY="lg">
                    <Stack gap="sm">
                      <span class="text-base-content/60 text-xs">
                        Container maxWidth="lg" paddingX="md" bg="base-300"
                      </span>
                      <Container
                        maxWidth="lg"
                        paddingX="md"
                        class="bg-base-300"
                      >
                        <Stack gap="md" align="center">
                          <h3 class="font-semibold text-lg">
                            Section + Container
                          </h3>
                          <p class="text-center text-base-content/70">
                            Section handles full-width background and vertical
                            padding. Container constrains content width and adds
                            horizontal padding.
                          </p>
                          <Button variant="primary">CTA</Button>
                        </Stack>
                      </Container>
                    </Stack>
                  </Section>
                </Stack>
              </Card.Body>
            </Card>
          </Stack>
        </Stack>
      </Stack>
    </DevLayout>
  );
}
