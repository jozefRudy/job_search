import type { Component, JSX } from "solid-js";
import { Button } from "~/components/ui/Button";
import { Container } from "~/components/ui/layout/Container";
import { Row } from "~/components/ui/layout/Row";
import { Section } from "~/components/ui/layout/Section";
import { Stack } from "~/components/ui/layout/Stack";

interface DevLayoutProps {
  title: string;
  backHref?: string;
  children: JSX.Element;
}

export const DevLayout: Component<DevLayoutProps> = (props) => {
  return (
    <Section bg="base-100" paddingY="sm" class="min-h-screen">
      <Container paddingX="sm">
        <Stack gap="lg">
          <Row align="center" gap="md">
            {props.backHref && (
              <Button
                variant="link"
                href={props.backHref}
                icon="back-arrow"
                iconPlacement="left"
                size="sm"
              >
                Back to Dev
              </Button>
            )}
            <h1 class="font-bold text-3xl">{props.title}</h1>
          </Row>
          {props.children}
        </Stack>
      </Container>
    </Section>
  );
};
