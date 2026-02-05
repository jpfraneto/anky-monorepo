import { useState } from "react";
import { fetchAPI } from "../api";

interface PricingModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function PricingModal({ isOpen, onClose }: PricingModalProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleCheckout = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetchAPI<{ checkoutUrl: string }>("/api/checkout", {
        successUrl: window.location.origin + "/gallery",
      });

      if (response.checkoutUrl) {
        window.location.href = response.checkoutUrl;
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create checkout");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />
      <div className="relative bg-gray-900 rounded-2xl border border-gray-700 p-8 max-w-md w-full mx-4">
        <button
          onClick={onClose}
          className="absolute top-4 right-4 text-gray-400 hover:text-white"
        >
          &times;
        </button>

        <h2 className="text-2xl font-bold text-white mb-2">Subscribe to Anky</h2>
        <p className="text-gray-400 mb-6">
          Your free session has been used. Subscribe to continue your writing practice.
        </p>

        <div className="bg-gray-800 rounded-xl p-6 mb-6">
          <div className="flex justify-between items-center mb-4">
            <span className="text-lg font-semibold text-white">Unlimited Writing</span>
            <span className="text-2xl font-bold text-amber-300">$9</span>
          </div>
          <p className="text-gray-400 text-sm">
            30 days of unlimited writing sessions with AI-generated art, reflections, and titles for every 8+ minute session.
          </p>
        </div>

        {error && (
          <p className="text-red-400 text-sm mb-4">{error}</p>
        )}

        <button
          onClick={handleCheckout}
          disabled={loading}
          className="w-full py-3 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-xl text-white font-semibold transition-colors"
        >
          {loading ? "Loading..." : "Subscribe - $9/month"}
        </button>

        <p className="text-gray-500 text-xs text-center mt-4">
          Powered by Polar.sh. Cancel anytime.
        </p>
      </div>
    </div>
  );
}
