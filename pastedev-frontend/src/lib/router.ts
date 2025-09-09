import { writable } from "svelte/store";

export interface Route {
  path: string;
  params: Record<string, string>;
}

function parseRoute(): Route {
  const pathname = window.location.pathname;

  if (pathname === "/") {
    return { path: "/", params: {} };
  }

  // Simple pattern matching for /{snippet_id} routes
  if (pathname.match(/^\/[a-zA-Z0-9_-]+$/)) {
    const id = pathname.slice(1);
    return { path: "/snippet", params: { id } };
  }

  return { path: pathname, params: {} };
}

export const currentRoute = writable<Route>(parseRoute());

function updateRoute() {
  currentRoute.set(parseRoute());
}

// Listen for browser navigation events
window.addEventListener("popstate", updateRoute);
window.addEventListener("load", updateRoute);

export function navigate(path: string) {
  window.history.pushState({}, "", path);
  updateRoute();
}
