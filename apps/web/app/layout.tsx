import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "SatGate",
  description: "Economic proof of intent for contact forms.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
