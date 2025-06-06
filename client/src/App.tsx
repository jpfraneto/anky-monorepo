import { useState, useEffect, useRef, useCallback, KeyboardEvent } from "react";
import { Button } from "./components/ui/button";
import type { AppType } from "server";
import { hc } from "hono/client";
import { Timer } from "lucide-react";
import DOMPurify from "dompurify";
import { sdk } from "@farcaster/frame-sdk";
import { motion, AnimatePresence } from "framer-motion";
import { usePrivy } from "@privy-io/react-auth";

const SERVER_URL = import.meta.env.VITE_API_URL;
const client = hc<AppType>(SERVER_URL);

const DEFAULT_SESSION_TIME = 8 * 60; // 8 min
const PAUSE_THRESHOLD = 8; // 8 s
const IDEAL_KEYSTROKE_INTERVAL = 180; // ms
const MAX_BLUR_PX = 4;

type WriteStatus = "idle" | "flow" | "finished" | "published";

interface AppProps {
  isLoggedIn?: boolean; // <-- wire your auth state here
}

function App({ isLoggedIn = false }: AppProps) {
  /* ─── state ─── */
  const [writing, setWriting] = useState("");
  const [isWriting, setIsWriting] = useState(false);
  const [response, setResponse] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [pauseWarning, setPauseWarning] = useState(0);
  const [totalWritingTime, setTotalWritingTime] = useState(0);
  const [isProcessing, setIsProcessing] = useState(false);
  const [keyboardOpen, setKeyboardOpen] = useState(false);
  const [viewportHeight, setViewportHeight] = useState(window.innerHeight);
  const [showSubmitButton, setShowSubmitButton] = useState(false);
  const [sessionCompleted, setSessionCompleted] = useState(false);
  const [confirmEarlySubmit, setConfirmEarlySubmit] = useState(false);
  const [confirmSessionComplete, setConfirmSessionComplete] = useState(false);
  const [backspaceCount, setBackspaceCount] = useState(0);
  const [keystrokeIntervals, setKeystrokeIntervals] = useState<number[]>([]);
  const [lastKeystrokeTime, setLastKeystrokeTime] = useState<number | null>(
    null
  );
  const [flowScore, setFlowScore] = useState(0);
  const [loginButtonHovered, setLoginButtonHovered] = useState(false);
  const [loginButtonClicked, setLoginButtonClicked] = useState(false);
  const [displayDescription, setDisplayDescription] = useState(true);
  const [currentKeystrokeInterval, setCurrentKeystrokeInterval] =
    useState<number>(IDEAL_KEYSTROKE_INTERVAL);

  /* ─── new state ─── */
  const [writeStatus, setWriteStatus] = useState<WriteStatus>("idle");
  const [hashId, setHashId] = useState<string | null>(null);
  const [placeholderHidden, setPlaceholderHidden] = useState(false);

  /* ─── streaming placeholder state ─── */
  const [placeholderText, setPlaceholderText] = useState("");
  const fullPlaceholder = "|";
  const [streamingComplete, setStreamingComplete] = useState(false);

  /* ─── refs ─── */
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const pauseTimerRef = useRef<NodeJS.Timeout | null>(null);
  const writingTimerRef = useRef<NodeJS.Timeout | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const lastWritingTimeRef = useRef<number>(Date.now());
  const sessionStartTimeRef = useRef<number | null>(null);
  const currentWritingRef = useRef<string>("");
  const streamIntervalRef = useRef<NodeJS.Timeout | null>(null);

  const { login } = usePrivy();

  /* ─── streaming text effect ─── */
  useEffect(() => {
    let i = 0;
    const speed = 50; // milliseconds per character

    // Clear any existing interval
    if (streamIntervalRef.current) {
      clearInterval(streamIntervalRef.current);
    }

    // Only start streaming if we're not writing yet
    if (!isWriting && !streamingComplete) {
      streamIntervalRef.current = setInterval(() => {
        if (i < fullPlaceholder.length) {
          setPlaceholderText(fullPlaceholder.substring(0, i + 1));
          i++;
        } else {
          clearInterval(streamIntervalRef.current as NodeJS.Timeout);
          setStreamingComplete(true);
        }
      }, speed);
    }

    return () => {
      if (streamIntervalRef.current) {
        clearInterval(streamIntervalRef.current);
      }
    };
  }, [isWriting, streamingComplete]);

  /* ─── flow-score calculation ─── */
  const calculateFlowScore = useCallback((intervals: number[]) => {
    if (!intervals.length) return 100;
    const avg = intervals.reduce((a, b) => a + b, 0) / intervals.length;
    const dev = Math.abs(avg - IDEAL_KEYSTROKE_INTERVAL);
    return Math.max(0, 100 - dev / 2);
  }, []);

  useEffect(() => {
    setFlowScore(calculateFlowScore(keystrokeIntervals));
    setInterval(() => {
      setPlaceholderHidden(!placeholderHidden);
    }, 444);
  }, [keystrokeIntervals, calculateFlowScore]);

  /* ─── initialize miniapp SDK ─── */
  useEffect(() => {
    sdk.actions
      .ready()
      .then(() => {
        setIsLoading(false);
      })
      .catch(console.error);
  }, []);

  /* ─── Update the currentWritingRef whenever writing changes ─── */
  useEffect(() => {
    currentWritingRef.current = writing;
  }, [writing]);

  /* ─── middleware key handler ─── */
  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      /* block Backspace */
      if (e.key === "Backspace") {
        e.preventDefault();
        setBackspaceCount((b) => b + 1);
        shakeArea();
        return;
      }

      /* cadence */
      const now = Date.now();
      if (lastKeystrokeTime !== null) {
        const diff = now - lastKeystrokeTime;
        if (diff > 0 && diff < 1000) {
          setKeystrokeIntervals((prev) => [...prev.slice(-50), diff]);
          setCurrentKeystrokeInterval(diff);
        }
      }
      setLastKeystrokeTime(now);
      lastWritingTimeRef.current = now;

      /* status transitions */
      if (writeStatus === "idle") setWriteStatus("flow");
      if (writeStatus === "finished") setWriteStatus("flow");
    },
    [lastKeystrokeTime, writeStatus]
  );

  const shakeArea = () => {
    textareaRef.current?.classList.add("border-red-500");
    textareaRef.current?.classList.add("bg-red-50");
    setTimeout(() => {
      textareaRef.current?.classList.remove("border-red-500");
      textareaRef.current?.classList.remove("bg-red-50");
    }, 250);
  };

  /* ─── viewport / keyboard detection ─── */
  useEffect(() => {
    const onResize = () => {
      const h = window.innerHeight;
      setKeyboardOpen(h < viewportHeight * 0.75);
      setViewportHeight(h);
    };
    window.addEventListener("resize", onResize);

    // For iOS devices that don't trigger resize on keyboard
    if (textareaRef.current) {
      textareaRef.current.addEventListener("focus", () => {
        setTimeout(() => setKeyboardOpen(true), 100);
      });
      textareaRef.current.addEventListener("blur", () => {
        setKeyboardOpen(false);
      });
    }

    return () => {
      window.removeEventListener("resize", onResize);
      if (textareaRef.current) {
        textareaRef.current.removeEventListener("focus", () =>
          setKeyboardOpen(true)
        );
        textareaRef.current.removeEventListener("blur", () =>
          setKeyboardOpen(false)
        );
      }
    };
  }, [viewportHeight]);

  /* ─── timers: writing elapsed + pause detection ─── */
  useEffect(() => {
    if (!isWriting) return cleanupTimers;

    writingTimerRef.current = setInterval(() => {
      if (!sessionStartTimeRef.current) return;
      const elapsed = Math.floor(
        (Date.now() - sessionStartTimeRef.current) / 1000
      );
      setTotalWritingTime(elapsed);
      if (elapsed >= DEFAULT_SESSION_TIME && !sessionCompleted) {
        setSessionCompleted(true);
        setWriteStatus("finished");
      }
    }, 1000);

    pauseTimerRef.current = setInterval(() => {
      const idle = (Date.now() - lastWritingTimeRef.current) / 1000;
      if (idle >= PAUSE_THRESHOLD && writeStatus === "flow") {
        setWriteStatus("finished");
        setShowSubmitButton(true);
      }
      if (idle >= PAUSE_THRESHOLD - 3 && idle < PAUSE_THRESHOLD) {
        setPauseWarning(PAUSE_THRESHOLD - Math.floor(idle));
      } else {
        setPauseWarning(0);
      }
    }, 250);

    return cleanupTimers;
  }, [isWriting, writeStatus, sessionCompleted]);

  const cleanupTimers = () => {
    writingTimerRef.current && clearInterval(writingTimerRef.current);
    pauseTimerRef.current && clearInterval(pauseTimerRef.current);
    timerRef.current && clearInterval(timerRef.current);
  };

  /* ─── blur strength ─── */
  const idleMs = Date.now() - lastWritingTimeRef.current;
  const idleFrac = Math.min(1, idleMs / (PAUSE_THRESHOLD * 1000));
  const blurPx = MAX_BLUR_PX * Math.pow(1 - flowScore / 100, 2) * idleFrac;

  /* ─── handlers ─── */
  const startWritingSession = () => {
    textareaRef.current?.focus();
    if (!isWriting) {
      // Only focus the textarea but don't start the writing session yet
      if (streamIntervalRef.current) {
        clearInterval(streamIntervalRef.current);
        setStreamingComplete(true);
        setPlaceholderText(fullPlaceholder); // Show the full placeholder text
      }
    }
  };

  const handleWriting = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newText = e.target.value;
    /* ignore sneaky backspace edits */
    if (newText.length < currentWritingRef.current.length) return;

    setWriting(newText);
    currentWritingRef.current = newText;
    lastWritingTimeRef.current = Date.now();

    // Reset confirmation state when user continues writing
    if (confirmEarlySubmit) {
      setConfirmEarlySubmit(false);
    }

    if (!isWriting && newText.trim()) {
      setIsWriting(true);
      setWriteStatus("flow");
      setTotalWritingTime(0);
      setSessionCompleted(false);
      sessionStartTimeRef.current = Date.now();
    }
    if (isWriting && !newText.trim()) {
      setIsWriting(false);
      setWriteStatus("idle");
      cleanupTimers();
      sessionStartTimeRef.current = null;
    }

    // Auto-scroll when reaching 60% of textarea height
    if (textareaRef.current) {
      const textarea = textareaRef.current;
      const scrollThreshold = textarea.scrollHeight * 0.6;

      if (textarea.scrollTop + textarea.clientHeight >= scrollThreshold) {
        textarea.scrollTop = 0;
      }
    }
  };

  const formatTime = (s: number) =>
    `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(
      2,
      "0"
    )}`;

  /* ─── hashing util ─── */
  const sha256 = async (txt: string) => {
    const buf = await crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(txt)
    );
    return Array.from(new Uint8Array(buf))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  };

  /* ─── publish flow ─── */
  const endSession = async () => {
    if (writeStatus === "published") return;

    // If session is not completed and confirmation is not shown yet
    if (!sessionCompleted && !confirmEarlySubmit) {
      setConfirmEarlySubmit(true);
      return;
    }

    // If session is completed but not confirmed yet
    if (sessionCompleted && !confirmSessionComplete && !confirmEarlySubmit) {
      setConfirmSessionComplete(true);
      return;
    }

    setIsWriting(false);
    setIsProcessing(true);
    setConfirmEarlySubmit(false);
    setConfirmSessionComplete(false);
    cleanupTimers();

    const finalText = currentWritingRef.current.trim();
    const finalTime = sessionStartTimeRef.current
      ? Math.floor((Date.now() - sessionStartTimeRef.current) / 1000)
      : totalWritingTime;

    try {
      /* remote log */
      const res = await client["writing-session"].$post({
        json: {
          writing: finalText,
          writingTime: finalTime,
          targetTime: DEFAULT_SESSION_TIME,
          flowScore,
          backspaceCount,
        },
      });

      if (!res.ok) {
        throw new Error("Failed to get response");
      }

      const data = await res.json();
      setResponse(`${data.message}`);

      /* hash + local / irys store */
      const h = await sha256(finalText);
      setHashId(h);

      if (isLoggedIn) {
        // await uploadToIrys(finalText, h);  // <-- plug in when ready
      } else {
        const prev = JSON.parse(localStorage.getItem("ankyWrites") || "{}");
        localStorage.setItem(
          "ankyWrites",
          JSON.stringify({ ...prev, [h]: { text: finalText, ts: Date.now() } })
        );
      }

      setWriteStatus("published");
    } catch (error) {
      console.error("Error:", error);
      setResponse(
        "Sorry, there was an error processing your writing. Please try again. And make sure next time you write for more than 8 minutes."
      );
    } finally {
      setIsProcessing(false);
      sessionStartTimeRef.current = null;
    }
  };

  const startNewSession = () => {
    setWriting("");
    currentWritingRef.current = "";
    setResponse(null);
    setTotalWritingTime(0);
    setPauseWarning(0);
    setIsProcessing(false);
    setSessionCompleted(false);
    setShowSubmitButton(false);
    setConfirmEarlySubmit(false);
    setConfirmSessionComplete(false);
    setBackspaceCount(0);
    setKeystrokeIntervals([]);
    setLastKeystrokeTime(null);
    setFlowScore(0);
    setWriteStatus("idle");
    sessionStartTimeRef.current = null;
    setIsWriting(false);
    // Reset the placeholder streaming effect
    setStreamingComplete(false);
    setPlaceholderText("");
    if (textareaRef.current) {
      textareaRef.current.focus();
    }
  };

  const handleLoginButtonClick = () => {
    // login();
    setLoginButtonClicked(true);
    setTimeout(() => {
      setLoginButtonClicked(false);
    }, 3000);
  };

  // Calculate the position of the flow indicator
  const calculateFlowIndicatorPosition = () => {
    if (!lastKeystrokeTime) return 50; // Center position when not typing

    // Calculate deviation from ideal keystroke interval
    const deviation = Math.abs(
      currentKeystrokeInterval - IDEAL_KEYSTROKE_INTERVAL
    );
    const maxDeviation = 300; // Maximum deviation to consider

    // Normalize to a percentage (0-100)
    const normalizedDeviation =
      Math.min(deviation, maxDeviation) / maxDeviation;

    // Convert to position (faster = left, slower = right)
    const position =
      currentKeystrokeInterval < IDEAL_KEYSTROKE_INTERVAL
        ? 50 - normalizedDeviation * 32 // Move left if typing faster
        : 50 + normalizedDeviation * 32; // Move right if typing slower

    return position;
  };

  const flowIndicatorPosition = calculateFlowIndicatorPosition();

  const timerDisplay = isWriting
    ? formatTime(totalWritingTime) // Show elapsed time when writing
    : formatTime(DEFAULT_SESSION_TIME); // Show target time before starting

  /* ─────────────────────────────── JSX ─────────────────────────────── */
  return (
    <div className="grow flex flex-col h-full bg-purple-50 px-4 sm:px-6 lg:px-8">
      <div
        className={`mx-auto w-full ${
          keyboardOpen ? "mb-0" : "mb-auto"
        } flex-grow flex flex-col`}
      >
        {/* ─── Header ─── */}
        {!isProcessing && (
          <div className="flex items-center justify-between my-2 sm:my-4">
            <div className="flex items-center gap-2">
              <div
                className={`${
                  confirmEarlySubmit && !sessionCompleted
                    ? "border-2 border-red-600 shadow-[0_0_15px_rgba(220,38,38,0.5)] animate-[pulse_0.8s_ease-in-out_infinite]"
                    : ""
                } relative flex items-center gap-2 bg-white/50 backdrop-blur-sm px-4 py-2 rounded-full border border-gray-200 shadow-sm`}
              >
                <Timer
                  className={`w-4 h-4 sm:w-5 sm:h-5 ${
                    confirmEarlySubmit && !sessionCompleted
                      ? "text-red-600"
                      : "text-gray-600"
                  }`}
                />
                <span
                  className={`text-xl sm:text-2xl ${
                    confirmEarlySubmit && !sessionCompleted
                      ? "text-red-600 font-bold"
                      : ""
                  }`}
                >
                  {timerDisplay}
                </span>
              </div>
              {pauseWarning > 0 && (
                <span
                  className={`text-red-500 text-sm sm:text-base ${
                    confirmEarlySubmit && !sessionCompleted
                      ? "font-bold animate-bounce"
                      : "animate-pulse"
                  }`}
                >
                  Keep writing! {pauseWarning}s
                </span>
              )}
            </div>

            {/* Right side buttons area */}
            <div className="flex items-center">
              {/* Submit button in top right during active session */}
              {isWriting && (showSubmitButton || sessionCompleted) && (
                <div className="flex flex-col items-end">
                  <div className="flex space-x-2">
                    <Button
                      onClick={endSession}
                      className={`bg-indigo-400 text-white border-black border shadow-lg hover:bg-indigo-500 w-28 ${
                        confirmEarlySubmit || confirmSessionComplete
                          ? "bg-indigo-400 "
                          : ""
                      }`}
                    >
                      {sessionCompleted
                        ? confirmSessionComplete
                          ? "Yes, Submit"
                          : "Complete Session"
                        : confirmEarlySubmit
                        ? "send"
                        : "send"}
                    </Button>
                    {(confirmEarlySubmit || confirmSessionComplete) && (
                      <Button
                        onClick={() => {
                          setConfirmEarlySubmit(false);
                          setConfirmSessionComplete(false);
                        }}
                        className="bg-red-500 text-white border-black border shadow-lg hover:bg-red-600 w-28"
                      >
                        write
                      </Button>
                    )}
                  </div>

                  {confirmSessionComplete && (
                    <p className="text-amber-600 text-sm mt-2">
                      Are you sure you want to submit your writing?
                    </p>
                  )}
                </div>
              )}

              {/* Only show login button when not in an active session */}
              {!isWriting && (
                <motion.button
                  className="px-4 py-2 rounded-lg text-white font-medium"
                  initial={{ backgroundColor: "#6366f1" }}
                  animate={
                    loginButtonClicked
                      ? {
                          backgroundColor: [
                            "#6366f1",
                            "#ec4899",
                            "#8b5cf6",
                            "#10b981",
                            "#f59e0b",
                            "#6366f1",
                          ],
                          rotate: [0, 5, -5, 5, -5, 0],
                          scale: [1, 1.1, 1, 1.1, 1],
                        }
                      : loginButtonHovered
                      ? { backgroundColor: "#4f46e5", scale: 1.05 }
                      : { backgroundColor: "#6366f1", scale: 1 }
                  }
                  transition={{
                    duration: loginButtonClicked ? 3 : 0.3,
                    repeat: loginButtonClicked ? 0 : undefined,
                  }}
                  onMouseEnter={() => setLoginButtonHovered(true)}
                  onMouseLeave={() => setLoginButtonHovered(false)}
                  onClick={handleLoginButtonClick}
                >
                  {loginButtonClicked ? "Soon..." : "Login"}
                </motion.button>
              )}
            </div>
          </div>
        )}

        {/* ─── Textarea ─── */}
        {!isProcessing && !response && (
          <div className="relative flex-1">
            {/* Flow indicator bar */}
            {isWriting && (
              <div className="absolute top-0 left-0 w-full h-2 bg-gray-100 rounded-t-lg z-10 overflow-hidden">
                {/* Optimal flow zone (36% width centered) */}
                <div className="absolute top-0 left-1/2 transform -translate-x-1/2 h-full w-[36%] bg-green-100 opacity-50" />

                {/* Flow indicator dot */}
                <motion.div
                  className="absolute top-0 h-full w-2 bg-red-500 rounded-full"
                  style={{
                    left: `${flowIndicatorPosition}%`,
                    transition: "left 0.1s ease-out",
                  }}
                  animate={{
                    scale: [1, 1.2, 1],
                    opacity: [0.7, 1, 0.7],
                  }}
                  transition={{
                    duration: 1,
                    repeat: Infinity,
                    repeatType: "reverse",
                  }}
                />
              </div>
            )}

            <motion.textarea
              ref={textareaRef}
              value={writing}
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck="false"
              autoComplete="off"
              onChange={handleWriting}
              onKeyDown={handleKeyDown}
              onClick={startWritingSession}
              placeholder={placeholderHidden ? "" : placeholderText}
              disabled={writeStatus === "published" || isProcessing}
              className={`w-full h-full resize-none p-3 outline-none font-serif ${
                isWriting
                  ? "border-0 bg-transparent focus:ring-0 focus:border-0"
                  : "border rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
              }`}
              style={{ filter: isWriting ? `blur(${blurPx}px)` : "none" }}
              initial={{ opacity: 0.8 }}
              animate={{
                opacity: 1,
                height: "16rem", // Keep the height consistent regardless of status
              }}
              transition={{ duration: 0.4 }}
            />

            {backspaceCount > 0 && (
              <div className="absolute top-0 right-0 mt-2 mr-2 text-sm text-red-600 bg-red-100 px-2 py-1 rounded shadow">
                {backspaceCount}
              </div>
            )}
          </div>
        )}

        {/* ─── Insight / processing ─── */}
        {(isProcessing || response) && (
          <div>
            {isProcessing && (
              <div className="mb-4 flex items-center gap-2">
                <svg
                  className="animate-spin h-5 w-5 text-indigo-600"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  ></circle>
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  ></path>
                </svg>
                <span className="text-indigo-700 font-medium">
                  Processing your writing...
                </span>
              </div>
            )}

            <div
              className="bg-gray-50 border rounded p-4 text-gray-700 whitespace-pre-wrap max-h-48 sm:max-h-64 overflow-y-auto hover:bg-gray-100 hover:border-indigo-300 transition-colors cursor-pointer"
              onClick={() => {
                navigator.clipboard.writeText(currentWritingRef.current || "");
              }}
              title="Click to copy your writing"
            >
              {currentWritingRef.current}
            </div>

            {!isProcessing && (
              <div className="mt-4 flex justify-between items-center">
                <div className="text-sm text-gray-600">
                  <div>
                    Flow Score:{" "}
                    <span className="font-bold text-indigo-600">
                      {Math.round(flowScore)}
                    </span>
                  </div>
                  <div>
                    Backspaces:{" "}
                    <span className="font-bold text-red-500">
                      {backspaceCount}
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {response && !keyboardOpen && (
          <motion.div
            className="bg-white rounded-lg shadow-xl p-4 sm:p-6 mt-4"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
          >
            <h2
              data-hash={hashId!}
              className="text-xl sm:text-2xl font-bold mb-2 sm:mb-4"
            >
              Anky Insights
            </h2>
            <div className="prose prose-indigo max-w-none text-sm sm:text-base">
              <div
                className="response-content"
                dangerouslySetInnerHTML={{
                  __html: DOMPurify.sanitize(response),
                }}
              />
            </div>
          </motion.div>
        )}

        {/* ─── Publish / Submit bar ─── */}
        {/* Removed this section since the Submit button has been moved to the top right */}

        {!isWriting && writing && !isLoading && !isProcessing && (
          <div className="mt-4 flex justify-end">
            <Button
              onClick={startNewSession}
              className="bg-indigo-600 hover:bg-indigo-700 text-white"
            >
              Start New Session
            </Button>
          </div>
        )}

        <AnimatePresence>
          {!isWriting && !response && !isProcessing && !keyboardOpen && (
            <motion.div
              className="text-center mt-12 mb-6 sm:mb-12"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.5 }}
            >
              <h1 className="text-4xl sm:text-5xl righteous font-black text-gray-900 mb-2 sm:mb-4">
                Anky
              </h1>

              <p
                onClick={() => {
                  setDisplayDescription(!displayDescription);
                }}
                className={`${
                  displayDescription ? "blur-sm" : ""
                } text-lg sm:text-xl handlee cursor-pointer text-gray-600 max-w-2xl mx-auto`}
              >
                A transformative writing practice designed to catalyze spiritual
                awakening through uninterrupted creative expression.
              </p>
              <motion.button
                className="p-4 mt-4 handlee text-xl rounded-xl bg-purple-400 border border-black"
                onClick={startWritingSession}
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
              >
                write 8 minutes
              </motion.button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default App;
