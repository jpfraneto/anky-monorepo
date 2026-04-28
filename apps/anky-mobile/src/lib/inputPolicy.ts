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
