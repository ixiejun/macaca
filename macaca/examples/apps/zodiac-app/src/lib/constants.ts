export interface ZodiacSign {
  name: string;
  symbol: string;
  dateRange: string;
  element: string;
  rulingPlanet: string;
  traits: string[];
  luckyNumbers: number[];
  luckyColors: string[];
  compatibility: string[];
  description: string;
}

export const ZODIAC_SIGNS: Record<string, ZodiacSign> = {
  aries: {
    name: "白羊座",
    symbol: "♈",
    dateRange: "3/21 - 4/19",
    element: "火",
    rulingPlanet: "火星",
    traits: ["勇敢", "自信", "热情", "直接", "冲动"],
    luckyNumbers: [1, 8, 17],
    luckyColors: ["红色", "橙色"],
    compatibility: ["狮子座", "射手座", "双子座", "水瓶座"],
    description: "白羊座是黄道十二宫的第一个星座，代表着新的开始和无畏的精神。他们天生具有领导才能，充满活��和竞争意识。"
  },
  taurus: {
    name: "金牛座",
    symbol: "♉",
    dateRange: "4/20 - 5/20",
    element: "土",
    rulingPlanet: "金星",
    traits: ["稳重", "务实", "耐心", "固执", "忠诚"],
    luckyNumbers: [2, 6, 9],
    luckyColors: ["绿色", "粉色"],
    compatibility: ["处女座", "摩羯座", "巨蟹座", "双鱼座"],
    description: "金牛座以稳定和可靠著称，他们欣赏生活中的美好事物，对艺术和美食有着天生的品味。"
  },
  gemini: {
    name: "双子座",
    symbol: "♊",
    dateRange: "5/21 - 6/21",
    element: "风",
    rulingPlanet: "水星",
    traits: ["机智", "好奇", "多变", "善于交流", "灵活"],
    luckyNumbers: [3, 5, 7],
    luckyColors: ["黄色", "浅蓝色"],
    compatibility: ["天秤座", "水瓶座", "白羊座", "狮子座"],
    description: "双子座拥有敏捷的思维和出色的沟通能力，他们对世界充满好奇，喜欢学习新事物和结识新朋友。"
  },
  cancer: {
    name: "巨蟹座",
    symbol: "♋",
    dateRange: "6/22 - 7/22",
    element: "水",
    rulingPlanet: "月亮",
    traits: ["敏感", "顾家", "情感丰富", "保护欲强", "直觉敏锐"],
    luckyNumbers: [2, 7, 11],
    luckyColors: ["银白色", "海蓝色"],
    compatibility: ["天蝎座", "双鱼座", "金牛座", "处女座"],
    description: "巨蟹座是最有家庭观念的星座，他们温柔体贴，善于照顾他人，对家人和朋友有着深厚的感情。"
  },
  leo: {
    name: "狮子座",
    symbol: "♌",
    dateRange: "7/23 - 8/22",
    element: "火",
    rulingPlanet: "太阳",
    traits: ["自信", "慷慨", "热情", "领导力强", "骄傲"],
    luckyNumbers: [1, 4, 19],
    luckyColors: ["金色", "橙色"],
    compatibility: ["白羊座", "射手座", "双子座", "天秤座"],
    description: "狮子座天生就是舞台上的明星，他们自信、大方、有魅力，总是能够吸引众人的目光。"
  },
  virgo: {
    name: "处女座",
    symbol: "♍",
    dateRange: "8/23 - 9/22",
    element: "土",
    rulingPlanet: "水星",
    traits: ["细心", "完美主义", "分析能力强", "实际", "谦虚"],
    luckyNumbers: [5, 14, 23],
    luckyColors: ["米色", "深绿色"],
    compatibility: ["金牛座", "摩羯座", "巨蟹座", "天蝎座"],
    description: "处女座以追求完美著称，他们注重细节，做事认真负责，是值得信赖的伙伴和同事。"
  },
  libra: {
    name: "天秤座",
    symbol: "♎",
    dateRange: "9/23 - 10/23",
    element: "风",
    rulingPlanet: "金星",
    traits: ["优雅", "公正", "和谐", "社交能力强", "犹豫不决"],
    luckyNumbers: [6, 15, 24],
    luckyColors: ["粉色", "淡蓝色"],
    compatibility: ["双子座", "水瓶座", "狮子座", "射手座"],
    description: "天秤座追求平衡与和谐，他们优雅、公正，擅长社交，是天生的外交家和调解者。"
  },
  scorpio: {
    name: "天蝎座",
    symbol: "♏",
    dateRange: "10/24 - 11/22",
    element: "水",
    rulingPlanet: "冥王星",
    traits: ["神秘", "深沉", "意志坚定", "洞察力强", "占有欲强"],
    luckyNumbers: [8, 13, 22],
    luckyColors: ["深红色", "黑色"],
    compatibility: ["巨蟹座", "双鱼座", "处女座", "摩羯座"],
    description: "天蝎座拥有强大的意志力和敏锐的洞察力，他们深沉神秘，一旦认定目标就会全力以赴。"
  },
  sagittarius: {
    name: "射手座",
    symbol: "♐",
    dateRange: "11/23 - 12/21",
    element: "火",
    rulingPlanet: "木星",
    traits: ["乐观", "自由", "冒险", "直率", "哲学思考"],
    luckyNumbers: [3, 9, 12],
    luckyColors: ["紫色", "深蓝色"],
    compatibility: ["白羊座", "狮子座", "天秤座", "水瓶座"],
    description: "射手座是黄道十二宫中的冒险家，他们热爱自由，追求真理，对生活充满乐观和热情。"
  },
  capricorn: {
    name: "摩羯座",
    symbol: "♑",
    dateRange: "12/22 - 1/19",
    element: "土",
    rulingPlanet: "土星",
    traits: ["务实", "有野心", "自律", "稳重", "悲观"],
    luckyNumbers: [4, 8, 13],
    luckyColors: ["黑色", "棕色"],
    compatibility: ["金牛座", "处女座", "天蝎座", "双鱼座"],
    description: "摩羯座是最有耐心和毅力的星座，他们脚踏实地，目标明确，是可靠的事业伙伴。"
  },
  aquarius: {
    name: "水瓶座",
    symbol: "♒",
    dateRange: "1/20 - 2/18",
    element: "风",
    rulingPlanet: "天王星",
    traits: ["独立", "创新", "人道主义", "叛逆", "理性"],
    luckyNumbers: [4, 7, 11],
    luckyColors: ["天蓝色", "银色"],
    compatibility: ["双子座", "天秤座", "白羊座", "射手座"],
    description: "水瓶座是最具创新精神的星座，他们思想前卫，追求自由和平等，常常有着独特的人生观。"
  },
  pisces: {
    name: "双鱼座",
    symbol: "♓",
    dateRange: "2/19 - 3/20",
    element: "水",
    rulingPlanet: "海王星",
    traits: ["浪漫", "富有同情心", "直觉强", "艺术天赋", "逃避现实"],
    luckyNumbers: [3, 7, 12],
    luckyColors: ["海蓝色", "淡紫色"],
    compatibility: ["巨蟹座", "天蝎座", "金牛座", "摩羯座"],
    description: "双鱼座是最富有想象力和同情心的星座，他们敏感细腻，有着丰富的内心世界和艺术天赋。"
  }
};

// 星座日期范围定义 (月/日)
const ZODIAC_DATES: { sign: string; startMonth: number; startDay: number; endMonth: number; endDay: number }[] = [
  { sign: "capricorn", startMonth: 12, startDay: 22, endMonth: 1, endDay: 19 },
  { sign: "aquarius", startMonth: 1, startDay: 20, endMonth: 2, endDay: 18 },
  { sign: "pisces", startMonth: 2, startDay: 19, endMonth: 3, endDay: 20 },
  { sign: "aries", startMonth: 3, startDay: 21, endMonth: 4, endDay: 19 },
  { sign: "taurus", startMonth: 4, startDay: 20, endMonth: 5, endDay: 20 },
  { sign: "gemini", startMonth: 5, startDay: 21, endMonth: 6, endDay: 21 },
  { sign: "cancer", startMonth: 6, startDay: 22, endMonth: 7, endDay: 22 },
  { sign: "leo", startMonth: 7, startDay: 23, endMonth: 8, endDay: 22 },
  { sign: "virgo", startMonth: 8, startDay: 23, endMonth: 9, endDay: 22 },
  { sign: "libra", startMonth: 9, startDay: 23, endMonth: 10, endDay: 23 },
  { sign: "scorpio", startMonth: 10, startDay: 24, endMonth: 11, endDay: 22 },
  { sign: "sagittarius", startMonth: 11, startDay: 23, endMonth: 12, endDay: 21 }
];

export function getZodiacSign(month: number, day: number): string | null {
  // 验证输入
  if (month < 1 || month > 12 || day < 1 || day > 31) {
    return null;
  }

  // 检查日期是否有效
  const daysInMonth = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (day > daysInMonth[month - 1]) {
    return null;
  }

  // 处理摩羯座跨年的特殊情况
  if ((month === 12 && day >= 22) || (month === 1 && day <= 19)) {
    return "capricorn";
  }

  // 查找对应的星座
  for (const dateInfo of ZODIAC_DATES) {
    if (dateInfo.sign === "capricorn") continue; // 跳过摩羯座，已经特殊处理

    if (month === dateInfo.startMonth && day >= dateInfo.startDay) {
      return dateInfo.sign;
    }
    if (month === dateInfo.endMonth && day <= dateInfo.endDay) {
      return dateInfo.sign;
    }
  }

  return null;
}

export function getZodiacInfo(sign: string): ZodiacSign | null {
  return ZODIAC_SIGNS[sign] || null;
}
