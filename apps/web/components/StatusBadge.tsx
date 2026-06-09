import { AlertTriangle, CheckCircle, Clock } from "lucide-react";

type StatusBadgeProps = {
  status: "paid" | "pending" | "failed" | string;
};

export function StatusBadge({ status }: StatusBadgeProps) {
  if (status === "paid") {
    return (
      <span className="inline-flex items-center gap-1 rounded-full border border-green-200 bg-green-50 px-2.5 py-1 text-xs font-semibold text-satGreen">
        <CheckCircle size={14} aria-hidden="true" />
        Paid
      </span>
    );
  }

  if (status === "failed") {
    return (
      <span className="inline-flex items-center gap-1 rounded-full border border-red-200 bg-red-50 px-2.5 py-1 text-xs font-semibold text-satRed">
        <AlertTriangle size={14} aria-hidden="true" />
        Failed
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-yellow-200 bg-yellow-50 px-2.5 py-1 text-xs font-semibold text-amber-700">
      <Clock size={14} aria-hidden="true" />
      Pending
    </span>
  );
}
