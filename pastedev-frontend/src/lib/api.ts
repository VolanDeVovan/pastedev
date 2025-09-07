// API base URL - in production this might come from environment variables
const API_BASE = "http://localhost:8080";

export class ApiError extends Error {
  status: number;

  constructor({ message, status }: { message: string; status: number }) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

export class ApiClient {
  static async getSnippet(id: string): Promise<string> {
    try {
      const response = await fetch(`${API_BASE}/api/snippets/${id}`);

      if (!response.ok) {
        throw new ApiError({
          message:
            response.status === 404
              ? "Snippet not found"
              : "Failed to fetch snippet",
          status: response.status,
        });
      }

      const content = await response.text();
      return content;
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }

      // Network or other errors
      throw new ApiError({
        message: "Network error: Unable to connect to server",
        status: 0,
      });
    }
  }

  static async createSnippet(content: string): Promise<string> {
    try {
      const response = await fetch(`${API_BASE}/api/snippets`, {
        method: "POST",
        headers: {
          "Content-Type": "text/plain",
        },
        body: content,
      });

      if (!response.ok) {
        throw new ApiError({
          message:
            response.status === 400
              ? "Content cannot be empty"
              : "Failed to create snippet",
          status: response.status,
        });
      }

      const snippetUrl = await response.text();
      return snippetUrl;
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }

      // Network or other errors
      throw new ApiError({
        message: "Network error: Unable to connect to server",
        status: 0,
      });
    }
  }
}
