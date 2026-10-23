import { createContext, useContext, useState, type ReactNode } from "react";
import { ConvexReactClient } from "convex/react";
import { ConvexProviderWithClerk } from "convex/react-clerk";
import { useAuth } from "@clerk/tanstack-react-start";

const BackendContext = createContext(false);
export const useBackendConfigured = () => useContext(BackendContext);

export function DataProvider({ children }: { children: ReactNode }) {
  const [client] = useState(() => {
    const url = import.meta.env.VITE_CONVEX_URL;
    return url
      ? new ConvexReactClient(url, { unsavedChangesWarning: false })
      : null;
  });
  if (!client)
    return (
      <BackendContext.Provider value={false}>
        {children}
      </BackendContext.Provider>
    );
  return (
    <BackendContext.Provider value={true}>
      <ConvexProviderWithClerk client={client} useAuth={useAuth}>
        {children}
      </ConvexProviderWithClerk>
    </BackendContext.Provider>
  );
}
