'use client';

import { ZodiacSign } from '@/lib/constants';

interface ZodiacCardProps {
  zodiac: ZodiacSign;
  onClose: () => void;
}

const elementColors: Record<string, string> = {
  '火': 'from-red-500 to-orange-500',
  '土': 'from-green-600 to-yellow-600',
  '风': 'from-cyan-400 to-blue-400',
  '水': 'from-blue-500 to-purple-500',
};

const elementBgColors: Record<string, string> = {
  '火': 'bg-red-500/20',
  '土': 'bg-green-500/20',
  '风': 'bg-cyan-500/20',
  '水': 'bg-blue-500/20',
};

export default function ZodiacCard({ zodiac, onClose }: ZodiacCardProps) {
  const gradientClass = elementColors[zodiac.element] || 'from-gray-500 to-gray-600';
  const bgClass = elementBgColors[zodiac.element] || 'bg-gray-500/20';

  return (
    <div className="zodiac-card rounded-2xl p-6 sm:p-8 w-full max-w-2xl mx-auto animate-fade-in">
      {/* 关闭按钮 */}
      <button
        onClick={onClose}
        className="absolute top-4 right-4 w-8 h-8 flex items-center justify-center rounded-full bg-white/10 hover:bg-white/20 transition-colors text-white/70 hover:text-white"
      >
        ✕
      </button>

      {/* 头部：星座符号和名称 */}
      <div className="text-center mb-6">
        <div className={`inline-flex items-center justify-center w-24 h-24 sm:w-32 sm:h-32 rounded-full ${bgClass} mb-4 animate-float`}>
          <span className="zodiac-symbol text-5xl sm:text-6xl">{zodiac.symbol}</span>
        </div>
        <h2 className="text-3xl sm:text-4xl font-bold gradient-text mb-2">{zodiac.name}</h2>
        <p className="text-white/60">{zodiac.dateRange}</p>
      </div>

      {/* 元素和守护星 */}
      <div className="flex justify-center gap-4 mb-6">
        <div className={`px-4 py-2 rounded-full bg-gradient-to-r ${gradientClass} text-white text-sm font-medium`}>
          {zodiac.element}象星座
        </div>
        <div className="px-4 py-2 rounded-full bg-white/10 text-white text-sm">
          守护星：{zodiac.rulingPlanet}
        </div>
      </div>

      {/* 描述 */}
      <p className="text-center text-white/80 mb-6 leading-relaxed">
        {zodiac.description}
      </p>

      {/* 详细信息网格 */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-6">
        {/* 性格特点 */}
        <div className="bg-white/5 rounded-xl p-4">
          <h3 className="text-lg font-semibold text-white mb-3 flex items-center gap-2">
            <span>✨</span> 性格特点
          </h3>
          <div className="flex flex-wrap gap-2">
            {zodiac.traits.map((trait, index) => (
              <span
                key={index}
                className={`px-3 py-1 rounded-full text-sm ${bgClass} text-white/90`}
              >
                {trait}
              </span>
            ))}
          </div>
        </div>

        {/* 幸运数字 */}
        <div className="bg-white/5 rounded-xl p-4">
          <h3 className="text-lg font-semibold text-white mb-3 flex items-center gap-2">
            <span>🔢</span> 幸运数字
          </h3>
          <div className="flex gap-2">
            {zodiac.luckyNumbers.map((num, index) => (
              <span
                key={index}
                className={`w-10 h-10 flex items-center justify-center rounded-full bg-gradient-to-r ${gradientClass} text-white font-bold`}
              >
                {num}
              </span>
            ))}
          </div>
        </div>

        {/* 幸运颜色 */}
        <div className="bg-white/5 rounded-xl p-4">
          <h3 className="text-lg font-semibold text-white mb-3 flex items-center gap-2">
            <span>🎨</span> 幸运颜色
          </h3>
          <div className="flex flex-wrap gap-2">
            {zodiac.luckyColors.map((color, index) => (
              <span
                key={index}
                className="px-3 py-1 rounded-full bg-white/10 text-white/90 text-sm"
              >
                {color}
              </span>
            ))}
          </div>
        </div>

        {/* 配对星座 */}
        <div className="bg-white/5 rounded-xl p-4">
          <h3 className="text-lg font-semibold text-white mb-3 flex items-center gap-2">
            <span>💕</span> 最佳配对
          </h3>
          <div className="flex flex-wrap gap-2">
            {zodiac.compatibility.map((sign, index) => (
              <span
                key={index}
                className={`px-3 py-1 rounded-full text-sm ${bgClass} text-white/90`}
              >
                {sign}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* 重新查询按钮 */}
      <div className="text-center">
        <button
          onClick={onClose}
          className="px-6 py-2 bg-white/10 hover:bg-white/20 rounded-lg text-white/80 hover:text-white transition-all"
        >
          查询其他星座
        </button>
      </div>
    </div>
  );
}
