import { NextRequest, NextResponse } from 'next/server';
import { getZodiacSign, getZodiacInfo } from '@/lib/constants';

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const monthParam = searchParams.get('month');
  const dayParam = searchParams.get('day');

  // 验证参数
  if (!monthParam || !dayParam) {
    return NextResponse.json(
      { error: '请提供月份和日期参数' },
      { status: 400 }
    );
  }

  const month = parseInt(monthParam, 10);
  const day = parseInt(dayParam, 10);

  // 验证数值有效性
  if (isNaN(month) || isNaN(day)) {
    return NextResponse.json(
      { error: '月份和日期必须是有效数字' },
      { status: 400 }
    );
  }

  // 计算星座
  const signKey = getZodiacSign(month, day);

  if (!signKey) {
    return NextResponse.json(
      { error: '无效的日期，请检查月份和日期是否正确' },
      { status: 400 }
    );
  }

  // 获取星座详细信息
  const zodiacInfo = getZodiacInfo(signKey);

  return NextResponse.json({
    success: true,
    data: {
      sign: signKey,
      ...zodiacInfo
    }
  });
}
