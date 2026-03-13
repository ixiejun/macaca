'use client';

import { useState } from 'react';
import DateSelector from '@/components/DateSelector';
import ZodiacCard from '@/components/ZodiacCard';
import StarBackground from '@/components/StarBackground';
import { ZodiacSign } from '@/lib/constants';

export default function Home() {
  const [zodiacData, setZodiacData] = useState<ZodiacSign | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleDateSelect = async (month: number, day: number) => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(`/api/zodiac?month=${month}&day=${day}`);
      const data = await response.json();

      if (!response.ok) {
        throw new Error(data.error || '查询失败，请重试');
      }

      setZodiacData(data.data);
    } catch (err) {
      setError(err instanceof Error ? err.message : '查询失败，请重试');
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    setZodiacData(null);
    setError(null);
  };

  return (
    <main className="relative min-h-screen flex flex-col items-center justify-center px-4 py-8 overflow-hidden">
      {/* 星星背景 */}
      <StarBackground />

      {/* 主内容 */}
      <div className="relative z-10 w-full max-w-4xl mx-auto">
        {/* 标题区域 */}
        <div className="text-center mb-12">
          <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold gradient-text mb-4">
            ✨ 星座查询 ✨
          </h1>
          <p className="text-white/60 text-lg sm:text-xl max-w-md mx-auto">
            选择您的出生日期，探索属于您的星座秘密
          </p>
        </div>

        {/* 日期选择器 */}
        {!zodiacData && (
          <div className="mb-8">
            <DateSelector onDateSelect={handleDateSelect} isLoading={isLoading} />
          </div>
        )}

        {/* 错误提示 */}
        {error && (
          <div className="mb-8 p-4 bg-red-500/20 border border-red-500/50 rounded-lg text-center text-red-300">
            {error}
          </div>
        )}

        {/* 加载动画 */}
        {isLoading && (
          <div className="flex flex-col items-center justify-center py-12">
            <div className="loader mb-4"></div>
            <p className="text-white/60">正在探索星空...</p>
          </div>
        )}

        {/* 星座卡片 */}
        {zodiacData && !isLoading && (
          <div className="relative">
            <ZodiacCard zodiac={zodiacData} onClose={handleClose} />
          </div>
        )}

        {/* 底部装饰 */}
        {!zodiacData && !isLoading && (
          <div className="mt-16 text-center">
            <div className="flex justify-center gap-4 text-3xl opacity-50">
              <span>♈</span>
              <span>♉</span>
              <span>♊</span>
              <span>♋</span>
              <span>♌</span>
              <span>♍</span>
            </div>
            <div className="flex justify-center gap-4 text-3xl opacity-50 mt-2">
              <span>♎</span>
              <span>♏</span>
              <span>♐</span>
              <span>♑</span>
              <span>♒</span>
              <span>♓</span>
            </div>
          </div>
        )}
      </div>

      {/* 页脚 */}
      <footer className="absolute bottom-4 text-center text-white/30 text-sm">
        <p>探索星空，了解自我</p>
      </footer>
    </main>
  );
}
