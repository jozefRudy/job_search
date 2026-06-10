import { createSignal } from "solid-js";
import { Button } from "~/components/ui/Button";
import { Card } from "~/components/ui/Card";
import { Stack } from "~/components/ui/layout/Stack";
import { ConfirmModal, Modal } from "~/components/ui/Modal";
import { DevLayout } from "../DevLayout";

export default function ModalPage() {
  const [basicOpen, setBasicOpen] = createSignal(false);
  const [confirmOpen, setConfirmOpen] = createSignal(false);
  const [confirmDangerOpen, setConfirmDangerOpen] = createSignal(false);
  const [lastAction, setLastAction] = createSignal<string>("none");

  return (
    <DevLayout title="Modal Kitchen Sink" backHref="/dev">
      <Stack gap="lg">
        <Stack gap="md">
          <h2 class="font-semibold text-xl">Basic Modal</h2>
          <Card>
            <Card.Body>
              <Stack gap="md">
                <Button variant="primary" onClick={() => setBasicOpen(true)}>
                  Open Modal
                </Button>
                <p class="text-base-content/60 text-sm">
                  Click backdrop or close button to dismiss.
                </p>
              </Stack>
            </Card.Body>
          </Card>
          <Modal
            open={basicOpen()}
            onClose={() => setBasicOpen(false)}
            title="Basic Modal"
            footer={
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setBasicOpen(false)}
              >
                Close
              </Button>
            }
          >
            <p>This is a basic modal with title, content, and footer.</p>
          </Modal>
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">ConfirmModal — Danger</h2>
          <Card>
            <Card.Body>
              <Stack gap="md">
                <Button
                  variant="danger"
                  onClick={() => setConfirmDangerOpen(true)}
                >
                  Delete something
                </Button>
                <p class="text-base-content/60 text-sm">
                  Last action: <span class="font-mono">{lastAction()}</span>
                </p>
              </Stack>
            </Card.Body>
          </Card>
          <ConfirmModal
            open={confirmDangerOpen()}
            onClose={() => setConfirmDangerOpen(false)}
            onConfirm={() => {
              setConfirmDangerOpen(false);
              setLastAction("confirmed delete");
            }}
            title="Delete item?"
            message="This will permanently remove the item. Cannot be undone."
            confirmText="Delete"
            confirmVariant="danger"
          />
        </Stack>

        <Stack gap="md">
          <h2 class="font-semibold text-xl">ConfirmModal — Primary</h2>
          <Card>
            <Card.Body>
              <Stack gap="md">
                <Button variant="primary" onClick={() => setConfirmOpen(true)}>
                  Publish post
                </Button>
                <p class="text-base-content/60 text-sm">
                  Last action: <span class="font-mono">{lastAction()}</span>
                </p>
              </Stack>
            </Card.Body>
          </Card>
          <ConfirmModal
            open={confirmOpen()}
            onClose={() => setConfirmOpen(false)}
            onConfirm={() => {
              setConfirmOpen(false);
              setLastAction("confirmed publish");
            }}
            title="Publish post?"
            message="This will make the post visible to everyone."
            confirmText="Publish"
            confirmVariant="primary"
          />
        </Stack>
      </Stack>
    </DevLayout>
  );
}
