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
        console.log(conn.peer_connection);
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

      <h3>Seeded files:</h3>
      <ul>
        {fileList.map((file, index) => (
          <li key={index}>{file.name}</li>
        ))}
      </ul>

      <form
        action={(formData) => {
          const addedFile = formData.get("file");
          if (addedFile instanceof File) {
            setFileList(fileList.concat(addedFile));
          }
        }}
      >
        <input type="file" name="file" id="file" />
        <button type="submit">Submit</button>
      </form>

      <form
        action={async (formData) => {
          const addedPeer = formData.get("peer_id");
          if (typeof addedPeer === "string") {
            const conn: Connection = await endpoint?.connect(addedPeer);
            console.log(conn.peer_connection);
          }
        }}
      >
        <input type="text" name="peer_id" id="peer_id" />
        <button type="submit">Submit</button>
      </form>
    </div>
  );
}
