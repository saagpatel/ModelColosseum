import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router";
import { AppShell } from "./components/AppShell";
import { Arena } from "./pages/Arena";
import { Leaderboard } from "./pages/Leaderboard";
import { History } from "./pages/History";
import "./index.css";

const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      {
        index: true,
        element: <Arena />,
      },
      {
        path: "leaderboard",
        element: <Leaderboard />,
      },
      {
        path: "history",
        element: <History />,
      },
    ],
  },
]);

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("root element not found");

createRoot(rootEl).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
