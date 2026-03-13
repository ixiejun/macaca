import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "星座查询 | Zodiac App",
  description: "输入您的出生日期，探索您的星座秘密",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN">
      <body className="font-sans">{children}</body>
    </html>
  );
}
