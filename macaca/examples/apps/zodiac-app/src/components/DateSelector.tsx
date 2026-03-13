'use client';

import { useState } from 'react';

interface DateSelectorProps {
  onDateSelect: (month: number, day: number) => void;
  isLoading: boolean;
}

const MONTHS = [
  { value: 1, label: '1月' },
  { value: 2, label: '2月' },
  { value: 3, label: '3月' },
  { value: 4, label: '4月' },
  { value: 5, label: '5月' },
  { value: 6, label: '6月' },
  { value: 7, label: '7月' },
  { value: 8, label: '8月' },
  { value: 9, label: '9月' },
  { value: 10, label: '10月' },
  { value: 11, label: '11月' },
  { value: 12, label: '12月' },
];

const DAYS_IN_MONTH = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

export default function DateSelector({ onDateSelect, isLoading }: DateSelectorProps) {
  const [month, setMonth] = useState<number>(1);
  const [day, setDay] = useState<number>(1);
  const [error, setError] = useState<string | null>(null);

  const days = Array.from({ length: DAYS_IN_MONTH[month - 1] }, (_, i) => i + 1);

  const handleMonthChange = (newMonth: number) => {
    setMonth(newMonth);
    // 如果当前日期超过了新月份的天数，调整日期
    if (day > DAYS_IN_MONTH[newMonth - 1]) {
      setDay(DAYS_IN_MONTH[newMonth - 1]);
    }
    setError(null);
  };

  const handleDayChange = (newDay: number) => {
    setDay(newDay);
    setError(null);
  };

  const handleSubmit = () => {
    if (month < 1 || month > 12) {
      setError('请选择有效的月份');
      return;
    }
    if (day < 1 || day > DAYS_IN_MONTH[month - 1]) {
      setError('请选择有效的日期');
      return;
    }
    setError(null);
    onDateSelect(month, day);
  };

  return (
    <div className="w-full max-w-md mx-auto">
      <div className="flex flex-col sm:flex-row gap-4 items-center justify-center">
        <div className="flex gap-2 items-center">
          <select
            value={month}
            onChange={(e) => handleMonthChange(Number(e.target.value))}
            className="date-select"
            disabled={isLoading}
          >
            {MONTHS.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </select>

          <select
            value={day}
            onChange={(e) => handleDayChange(Number(e.target.value))}
            className="date-select"
            disabled={isLoading}
          >
            {days.map((d) => (
              <option key={d} value={d}>
                {d}日
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={handleSubmit}
          disabled={isLoading}
          className="px-6 py-3 bg-gradient-to-r from-purple-500 to-pink-500 rounded-lg font-medium text-white hover:from-purple-600 hover:to-pink-600 transition-all duration-300 disabled:opacity-50 disabled:cursor-not-allowed shadow-lg hover:shadow-xl transform hover:-translate-y-0.5"
        >
          {isLoading ? (
            <span className="flex items-center gap-2">
              <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24">
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                  fill="none"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
              查询中...
            </span>
          ) : (
            '查询星座'
          )}
        </button>
      </div>

      {error && (
        <p className="mt-3 text-center text-red-400 text-sm">{error}</p>
      )}
    </div>
  );
}
