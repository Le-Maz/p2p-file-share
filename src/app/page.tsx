"use client";

import { useEffect, useState } from "react";
import type { Connection, Endpoint } from "../../iroh-wrapper/pkg/iroh_wrapper";

function Loading() {
  return <div>Loading...</div>;
}

export default function Home() {
  const [endpoint, setEndpoint] = useState<Endpoint>();
  const [fileList, setFileList] = useState<File[]>([]);

  useEffect(() => {
    (async () => {
      const { Endpoint } = await import("../../iroh-wrapper/pkg/iroh_wrapper");
      const endpoint = await Endpoint.new();
      await endpoint.initialized();
      setEndpoint(endpoint);
      endpoint.listen(async (conn: Connection) => {
        conn.peer_connection.createDataChannel("something");
        let name: string;
        conn.data_channel.onmessage = (message) => {
          if (typeof message.data === "string") {
            name = message.data;
            conn.data_channel.send("PONG");
          } else {
            const anchor = document.createElement("a");
            anchor.download = name;
            anchor.href = URL.createObjectURL(message.data as Blob);
            anchor.click();
          }
        };
      });
    })();
    return () => {
      endpoint?.free();
    };
  }, []);

  if (endpoint === undefined) {
    return <Loading />;
  }

  return (
    <div>
      <h3>Node ID: {endpoint.node_id()}</h3>

      <form
        action={async (formData) => {
          const addedPeer = formData.get("peer_id") as string;
          const file = formData.get("file") as File;
          const conn: Connection = await endpoint?.connect(addedPeer);
          const pingInterval = setInterval(
            () => conn.data_channel.send(file.name),
            300
          );
          conn.data_channel.onmessage = async (message) => {
            clearInterval(pingInterval);
            conn.data_channel.send(await file.arrayBuffer());
          };
        }}
      >
        <input type="file" name="file" id="file" />
        <input type="text" name="peer_id" id="peer_id" />
        <button type="submit">Submit</button>
      </form>
    </div>
  );
}
