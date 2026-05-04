import { isAcceptedCharacter } from "./ankyProtocol";

export type AnkyInputPolicy = {
  accessibilityInput: string;
  autocorrect: string;
  editing: string;
  enter: string;
  imeComposition: string;
  paste: string;
  smartPunctuation: string;
  textSubstitution: string;
  voiceInput: string;
};

export const ANKY_INPUT_POLICY: AnkyInputPolicy = {
  accessibilityInput:
    "not intentionally blocked, but only committed single characters that advance the stream are accepted",
  autocorrect: "disabled where React Native exposes controls",
  editing: "backspace, delete, arrows, selection edits, and replacements are rejected",
  enter: "disabled; multiline input is not used",
  imeComposition:
    "accepted only after the platform commits exactly one Unicode character to the hidden input",
  paste: "disabled by hidden context menu and rejected when more than one character arrives",
  smartPunctuation: "disabled where the platform exposes controls; multi-character substitutions are rejected",
  textSubstitution: "not intentionally supported; replacements are rejected unless they commit one character",
  voiceInput: "not intentionally supported",
};

export type AnkyInputDecision =
  | {
      accepted: true;
      char: string;
    }
  | {
      accepted: false;
      reason:
        | "deletion"
        | "multi_character"
        | "replacement"
        | "same_value"
        | "unsupported_character";
    };

export function getAcceptedInputCharacter(
  previousValue: string,
  nextValue: string,
): AnkyInputDecision {
  if (nextValue === previousValue) {
    return { accepted: false, reason: "same_value" };
  }

  if (nextValue.length < previousValue.length) {
    return { accepted: false, reason: "deletion" };
  }

  if (!nextValue.startsWith(previousValue)) {
    return { accepted: false, reason: "replacement" };
  }

  const inserted = nextValue.slice(previousValue.length);
  const characters = Array.from(inserted);

  if (characters.length !== 1 || characters[0] !== inserted) {
    return { accepted: false, reason: "multi_character" };
  }

  const char = characters[0];

  if (!isAcceptedCharacter(char)) {
    return { accepted: false, reason: "unsupported_character" };
  }

  return { accepted: true, char };
}
