import { describe, expect, it } from "vitest";

import { closeSession, computeSessionHashSync } from "../ankyProtocol";
import {
  buildAnkyCarpet,
  buildCarpetFromAnkyStrings,
  createProcessingCarpetPayload,
  createProcessingTicketRequest,
} from "./carpet";

describe("anky carpet processing", () => {
  it("builds a carpet from local .anky strings", () => {
    const raw = closeSession("1000 a\n");
    const carpet = buildCarpetFromAnkyStrings("reflection", [raw], 1700000000000);

    expect(carpet).toEqual({
      carpetVersion: 1,
      createdAt: 1700000000000,
      entries: [
        {
          anky: raw,
          sessionHash: computeSessionHashSync(raw),
        },
      ],
      purpose: "reflection",
    });
  });

  it("rejects a carpet entry whose hash does not match the .anky bytes", () => {
    const raw = closeSession("1000 a\n");

    expect(() =>
      buildAnkyCarpet(
        "reflection",
        [
          {
            anky: raw,
            sessionHash: "0".repeat(64),
          },
        ],
        1700000000000,
      ),
    ).toThrow("does not match");
  });

  it("creates a typed ticket request without raw .anky content", () => {
    const raw = closeSession("1000 a\n");
    const carpet = buildCarpetFromAnkyStrings("image", [raw], 1700000000000);

    expect(createProcessingTicketRequest(carpet)).toEqual({
      estimatedEntryCount: 1,
      processingType: "image",
      sessionHashes: [computeSessionHashSync(raw)],
    });
  });

  it("only allows plaintext carpet payloads behind the dev flag", () => {
    const raw = closeSession("1000 a\n");
    const carpet = buildCarpetFromAnkyStrings("reflection", [raw], 1700000000000);

    expect(() =>
      createProcessingCarpetPayload(carpet, {
        devPlaintextProcessingAllowed: false,
      }),
    ).toThrow("unavailable");

    expect(
      createProcessingCarpetPayload(carpet, {
        devPlaintextProcessingAllowed: true,
      }),
    ).toMatchObject({
      encryptionScheme: "dev_plaintext",
    });
  });
});
