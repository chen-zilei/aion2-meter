import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Overlay } from "./Overlay";
import "../shared/styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
);
