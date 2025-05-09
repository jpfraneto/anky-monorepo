import { Link } from "react-router";

export default function Sessions() {
  return (
    <div className="container mx-auto py-10">
      <div className="rounded-lg border bg-card text-card-foreground shadow-sm">
        <div className="flex flex-col space-y-1.5 p-6">
          <h3 className="text-2xl font-semibold leading-none tracking-tight">
            Writing Sessions
          </h3>
          <p className="text-sm text-muted-foreground">
            View your past writing sessions and analytics
          </p>
        </div>
        <div className="p-6 pt-0">
          <div className="flex flex-col space-y-4">
            <p className="text-muted-foreground">
              No sessions found. Start writing to create your first session.
            </p>
            <Link
              to="/"
              className="inline-flex items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground ring-offset-background transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
            >
              Start a new writing session
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}
