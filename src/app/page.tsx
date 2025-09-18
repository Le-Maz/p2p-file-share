"use client";

import { useEffect, useState } from "react";
import type { Endpoint } from "../../iroh-wrapper/pkg/iroh_wrapper";

function Loading() {
  return <div>Loading...</div>;
}

export default function Home() {
  const [endpoint, setEndpoint] = useState<Endpoint>();
  const [initialized, setInitialized] = useState<boolean>(false);

  useEffect(() => {
    (async () => {
      const { Endpoint } = await import("../../iroh-wrapper/pkg/iroh_wrapper");
      const endpoint = await Endpoint.new();
      setEndpoint(endpoint);
      await endpoint.initialized();
      setInitialized(true);
    })();
  }, []);

  if (endpoint === undefined) {
    return <Loading />;
  }

  return (
    <div>
      <h3>Node ID: {endpoint.node_id()}</h3>
      <p>Initialized: {initialized ? "true" : "false"}</p>
    </div>
  );
}
