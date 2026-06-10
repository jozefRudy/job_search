import { type Component, type JSX, Show } from "solid-js";
import { Button } from "./Button";
import { Row } from "./layout/Row";

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children?: JSX.Element;
  footer?: JSX.Element;
}

export const Modal: Component<ModalProps> = (props) => {
  return (
    <Show when={props.open}>
      <dialog
        class="modal modal-open"
        onClick={(e) => {
          if (e.target === e.currentTarget) props.onClose();
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") props.onClose();
        }}
      >
        <div class="modal-box">
          <Show when={props.title}>
            <h3 class="font-bold text-lg">{props.title}</h3>
          </Show>
          <div class="py-4">{props.children}</div>
          <Show when={props.footer}>
            <div class="modal-action">{props.footer}</div>
          </Show>
        </div>
      </dialog>
    </Show>
  );
};

export interface ConfirmModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title?: string;
  message?: string;
  confirmText?: string;
  cancelText?: string;
  confirmVariant?: "primary" | "danger";
}

export const ConfirmModal: Component<ConfirmModalProps> = (props) => {
  const title = () => props.title ?? "Are you sure?";
  const message = () => props.message ?? "This action cannot be undone.";
  const confirmText = () => props.confirmText ?? "Confirm";
  const cancelText = () => props.cancelText ?? "Cancel";
  const confirmVariant = () => props.confirmVariant ?? "danger";

  return (
    <Modal
      open={props.open}
      onClose={props.onClose}
      title={title()}
      footer={
        <Row gap="sm" justify="end">
          <Button variant="ghost" size="sm" onClick={props.onClose}>
            {cancelText()}
          </Button>
          <Button
            variant={confirmVariant()}
            size="sm"
            onClick={props.onConfirm}
          >
            {confirmText()}
          </Button>
        </Row>
      }
    >
      <p>{message()}</p>
    </Modal>
  );
};
