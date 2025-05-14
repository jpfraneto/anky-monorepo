import { useState, useEffect, useRef } from "react";
import { Button } from "./components/ui/button";
import type { AppType } from "server";
import { hc } from "hono/client";
import { Timer } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeRaw from "rehype-raw";

const SERVER_URL = import.meta.env.VITE_API_URL;
const client = hc<AppType>(SERVER_URL);

const DEFAULT_SESSION_TIME = 8 * 60; // 8 minutes in seconds
const PAUSE_THRESHOLD = 8; // 8 seconds pause threshold

function App() {
  const [writing, setWriting] = useState("");
  const [isWriting, setIsWriting] = useState(false);
  const [response, setResponse] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [pauseWarning, setPauseWarning] = useState(0);
  const [totalWritingTime, setTotalWritingTime] = useState(0);
  const [isProcessing, setIsProcessing] = useState(false);
  const [keyboardOpen, setKeyboardOpen] = useState(false);
  const [viewportHeight, setViewportHeight] = useState(window.innerHeight);
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const pauseTimerRef = useRef<NodeJS.Timeout | null>(null);
  const writingTimerRef = useRef<NodeJS.Timeout | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const lastWritingTimeRef = useRef<number>(Date.now());
  const sessionStartTimeRef = useRef<number | null>(null);
  // Add a ref to store the current writing content to avoid closure issues
  const currentWritingRef = useRef<string>("");

  // Cleanup function for timers
  const cleanupTimers = () => {
    if (writingTimerRef.current) clearInterval(writingTimerRef.current);
    if (pauseTimerRef.current) clearInterval(pauseTimerRef.current);
    if (timerRef.current) clearInterval(timerRef.current);
  };

  // Update the currentWritingRef whenever writing changes
  useEffect(() => {
    currentWritingRef.current = writing;
  }, [writing]);

  // Handle viewport height changes (for mobile keyboard)
  useEffect(() => {
    const handleResize = () => {
      const newHeight = window.innerHeight;
      // If the height decreased significantly, keyboard is likely open
      if (newHeight < viewportHeight * 0.75) {
        setKeyboardOpen(true);
      } else {
        setKeyboardOpen(false);
      }
      setViewportHeight(newHeight);
    };

    window.addEventListener("resize", handleResize);

    // For iOS devices that don't trigger resize on keyboard
    if (textareaRef.current) {
      textareaRef.current.addEventListener("focus", () => {
        // Small delay to ensure the keyboard is fully open
        setTimeout(() => setKeyboardOpen(true), 100);
      });
      textareaRef.current.addEventListener("blur", () => {
        setKeyboardOpen(false);
      });
    }

    return () => {
      window.removeEventListener("resize", handleResize);
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

  useEffect(() => {
    if (isWriting) {
      // Set session start time when writing begins
      if (!sessionStartTimeRef.current) {
        sessionStartTimeRef.current = Date.now();
      }

      // Start the writing timer
      writingTimerRef.current = setInterval(() => {
        if (sessionStartTimeRef.current) {
          const elapsedTime = Math.floor(
            (Date.now() - sessionStartTimeRef.current) / 1000
          );
          setTotalWritingTime(elapsedTime);
        }
      }, 1000);

      // Start the pause detection timer
      pauseTimerRef.current = setInterval(() => {
        const now = Date.now();
        const timeSinceLastWrite = (now - lastWritingTimeRef.current) / 1000;

        if (timeSinceLastWrite >= PAUSE_THRESHOLD) {
          endSession();
        } else if (timeSinceLastWrite >= PAUSE_THRESHOLD - 3) {
          setPauseWarning(PAUSE_THRESHOLD - Math.floor(timeSinceLastWrite));
        } else {
          setPauseWarning(0);
        }
      }, 100);
    }

    return cleanupTimers;
  }, [isWriting]);

  const endSession = async () => {
    // Capture the current writing content before changing any state
    const finalWritingContent = currentWritingRef.current;

    setIsWriting(false);
    setIsProcessing(true);
    cleanupTimers();

    // Calculate final writing time
    let finalTime = totalWritingTime;
    if (sessionStartTimeRef.current) {
      finalTime = Math.floor((Date.now() - sessionStartTimeRef.current) / 1000);
      setTotalWritingTime(finalTime);
    }

    // Pass the captured writing content to handleSubmit
    await handleSubmit(finalWritingContent, finalTime);
  };

  const handleWriting = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newText = e.target.value;
    setWriting(newText);
    // Also update the ref directly to ensure it's always current
    currentWritingRef.current = newText;
    lastWritingTimeRef.current = Date.now();

    // Start session when user starts writing
    if (!isWriting && newText.trim()) {
      setIsWriting(true);
      setTotalWritingTime(0);
      sessionStartTimeRef.current = Date.now();
    }

    // End session if text is cleared
    if (isWriting && !newText.trim()) {
      setIsWriting(false);
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

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs
      .toString()
      .padStart(2, "0")}`;
  };

  // Modified to accept content and time as parameters
  const handleSubmit = async (
    contentToSubmit?: string,
    writingTimeToSubmit?: number
  ) => {
    // Use provided content or fall back to current state
    const textToSend = contentToSubmit || currentWritingRef.current;

    if (!textToSend.trim()) return;

    setIsLoading(true);
    try {
      // Use provided time or calculate from session start
      const finalWritingTime =
        writingTimeToSubmit ||
        (sessionStartTimeRef.current
          ? Math.floor((Date.now() - sessionStartTimeRef.current) / 1000)
          : totalWritingTime);

      console.log("WRITING", textToSend);
      console.log("WRITING LENGTH", textToSend.length);
      console.log("TOTAL WRITING TIME", finalWritingTime);
      console.log("DEFAULT SESSION TIME", DEFAULT_SESSION_TIME);

      const res = await client["writing-session"].$post({
        json: {
          writing: textToSend,
          writingTime: finalWritingTime,
          targetTime: DEFAULT_SESSION_TIME,
        },
      });

      if (!res.ok) {
        throw new Error("Failed to get response");
      }

      const data = await res.json();
      setResponse(data.message);
    } catch (error) {
      console.error("Error:", error);
      setResponse(
        "Sorry, there was an error processing your writing. Please try again. And make sure next time you write for more than 8 minutes."
      );
    } finally {
      setIsLoading(false);
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
    sessionStartTimeRef.current = null;
    if (textareaRef.current) {
      textareaRef.current.focus();
    }
  };

  return (
    <div className="grow flex flex-col h-full py-4 px-4 sm:py-12 sm:px-6 lg:px-8">
      <div
        className={`max-w-4xl mx-auto w-full ${
          keyboardOpen ? "mb-0" : "mb-auto"
        } flex-grow flex flex-col`}
      >
        {!isWriting && !response && !isProcessing && !keyboardOpen && (
          <div className="text-center mb-6 sm:mb-12">
            <h1 className="text-4xl sm:text-5xl font-black text-gray-900 mb-2 sm:mb-4">
              Anky
            </h1>
            <p className="text-lg sm:text-xl text-gray-600 max-w-2xl mx-auto">
              A transformative writing practice designed to catalyze spiritual
              awakening through uninterrupted creative expression.
            </p>
          </div>
        )}

        <div
          className={`bg-white rounded-lg shadow-xl p-4 sm:p-6 mb-4 sm:mb-8 ${
            keyboardOpen ? "flex-grow" : ""
          }`}
        >
          {isWriting && (
            <div className="flex items-center justify-between mb-2 sm:mb-4">
              <div className="flex items-center gap-2 sm:gap-4">
                <div className="flex items-center gap-1 sm:gap-2">
                  <Timer className="w-4 h-4 sm:w-5 sm:h-5 text-gray-500" />
                  <span className="text-xl sm:text-2xl font-mono">
                    {formatTime(totalWritingTime)}
                  </span>
                </div>
                {pauseWarning > 0 && (
                  <div className="text-red-500 text-sm sm:text-base font-medium animate-pulse">
                    Keep writing! {pauseWarning}s left...
                  </div>
                )}
              </div>
            </div>
          )}

          {!isProcessing && !response && (
            <textarea
              ref={textareaRef}
              value={writing}
              onChange={handleWriting}
              disabled={isProcessing || !!response}
              placeholder="Just write, life will do the rest..."
              className={`w-full p-3 sm:p-4 text-base sm:text-lg border rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 ${
                keyboardOpen ? "h-[50vh]" : "h-48 sm:h-64"
              }`}
              autoFocus
            />
          )}

          {(isProcessing || !!response) && (
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

              <div className="bg-gray-50 border rounded p-4 text-gray-700 whitespace-pre-wrap max-h-48 sm:max-h-64 overflow-y-auto">
                {currentWritingRef.current}
              </div>
            </div>
          )}

          {!isWriting && writing && !isLoading && !isProcessing && (
            <div className="mt-4 flex justify-end">
              <Button
                onClick={startNewSession}
                className="bg-indigo-600 hover:bg-indigo-700"
              >
                Start New Session
              </Button>
            </div>
          )}
        </div>

        {response && !keyboardOpen && (
          <div className="bg-white rounded-lg shadow-xl p-4 sm:p-6">
            <h2 className="text-xl sm:text-2xl font-bold mb-2 sm:mb-4">
              Anky Insights
            </h2>
            <div className="prose max-w-none text-sm sm:text-base">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                rehypePlugins={[rehypeRaw]}
                className="markdown-content"
              >
                {response}
              </ReactMarkdown>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
