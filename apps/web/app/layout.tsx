import type { Metadata } from "next";
import { Comfortaa } from "next/font/google";
import "./globals.css";

const comfortaa = Comfortaa({
  subsets: ["latin"],
  variable: "--font-brand",
});

export const metadata: Metadata = {
  title: "SatGate | Economic proof of intent for contact forms",
  description: "Lightning-powered contact forms that make abuse costly and keep verified messages easy to read.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={comfortaa.variable}>
      <body>{children}</body>
    </html>
  );
}
