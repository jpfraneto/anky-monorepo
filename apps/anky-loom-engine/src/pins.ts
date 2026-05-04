import { LoomGeometry } from "./geometry";
import type { FlowBucket } from "./rhythm";
import type { LoomThread } from "./thread";

export type LoomPinActivationState = "empty" | "visited" | "origin";

export interface LoomPinState {
  day: number;
  pinIndex: number;
  state: LoomPinActivationState;
  visitCount: number;
  originSessionHash?: string;
  originFlowBucket?: FlowBucket;
}

export function buildPinStates(geometry: LoomGeometry, threads: readonly LoomThread[]): LoomPinState[] {
  const pinStates = geometry.anchors.map<LoomPinState>((anchor) => ({
    day: anchor.dayIndex,
    pinIndex: anchor.anchorIndex,
    state: "empty",
    visitCount: 0,
  }));
  const byPinIndex = new Map(pinStates.map((pinState) => [pinState.pinIndex, pinState]));

  for (const thread of threads) {
    const originPinIndex = thread.route[0];
    const originPin = byPinIndex.get(originPinIndex);

    if (originPin) {
      originPin.state = "origin";
      originPin.originSessionHash ??= thread.sessionHash;
      originPin.originFlowBucket ??= thread.flowBucket;
    }

    for (const pinIndex of thread.route) {
      if (pinIndex === originPinIndex) {
        continue;
      }

      const pinState = byPinIndex.get(pinIndex);
      if (!pinState) {
        continue;
      }

      pinState.visitCount += 1;
      if (pinState.state === "empty") {
        pinState.state = "visited";
      }
    }
  }

  return pinStates;
}
