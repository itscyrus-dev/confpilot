import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const alertVariants = cva("relative w-full rounded-lg border px-4 py-3 text-sm grid gap-1", {
  variants: {
    variant: {
      default: "bg-card text-card-foreground",
      warning: "border-amber-200 bg-amber-50 text-amber-900",
      destructive: "border-destructive/50 text-destructive bg-card"
    }
  },
  defaultVariants: {
    variant: "default"
  }
});

function Alert({ className, variant, ...props }: React.ComponentProps<"div"> & VariantProps<typeof alertVariants>) {
  return <div data-slot="alert" role="alert" className={cn(alertVariants({ variant }), className)} {...props} />;
}

function AlertTitle({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="alert-title" className={cn("font-medium leading-none", className)} {...props} />;
}

function AlertDescription({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="alert-description" className={cn("text-sm leading-relaxed", className)} {...props} />;
}

export { Alert, AlertTitle, AlertDescription };

