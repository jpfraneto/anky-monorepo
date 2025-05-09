import { Outlet, useNavigate } from "react-router-dom";
import { usePrivy } from "@privy-io/react-auth";
import { useEffect, useState } from "react";

const Layout = () => {
  const { login, authenticated, user, ready } = usePrivy();
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    // Check if user is authenticated after Privy is ready
    if (ready && authenticated && user) {
      // Send user details to backend
      sendUserDetailsToBackend(user);
    }
  }, [ready, authenticated, user]);

  const sendUserDetailsToBackend = async (user: any) => {
    try {
      // Example API call to your backend
      const response = await fetch(
        `${import.meta.env.VITE_API_URL}/auth/login`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            user,
            message: "the user was logged in successfully",
          }),
        }
      );

      if (!response.ok) {
        throw new Error("Failed to register/login user with backend");
      }

      // Optional: handle response from backend
      const data = await response.json();
      console.log("the data from the backend is", data);
    } catch (error) {
      console.error("Error sending user details to backend:", error);
    }
  };

  const handleUserLogin = async () => {
    try {
      setIsLoading(true);
      // Handle privy user login
      await login();

      // Note: We don't need to manually check authentication here
      // The useEffect above will handle sending user details to backend
      // once Privy updates the authenticated and user states
    } catch (error) {
      console.error("Login error:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleProfileClick = () => {
    navigate("/profile");
  };

  return (
    <div className="min-h-screen">
      <nav className="bg-white shadow-md">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16 items-center">
            <div className="flex-shrink-0">
              <h1 className="text-xl font-bold">Anky</h1>
            </div>
            <div>
              {ready && authenticated ? (
                <div className="flex space-x-4">
                  <span className="self-center">
                    {user?.email?.address ||
                      user?.wallet?.address?.substring(0, 6) + "..." ||
                      "User"}
                  </span>
                  <button
                    className="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded-md text-sm font-medium"
                    onClick={handleProfileClick}
                  >
                    Profile
                  </button>
                </div>
              ) : (
                <button
                  className="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-md text-sm font-medium"
                  onClick={handleUserLogin}
                  disabled={isLoading}
                >
                  {isLoading ? "Logging in..." : "Login"}
                </button>
              )}
            </div>
          </div>
        </div>
      </nav>
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        <Outlet />
      </main>
    </div>
  );
};

export default Layout;
