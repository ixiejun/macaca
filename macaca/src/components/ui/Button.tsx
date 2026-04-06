import * as React from "react";

// 定义按钮变体类型
type ButtonVariant = 
  | "primary" 
  | "secondary" 
  | "outline" 
  | "ghost" 
  | "destructive"
  | "link";

// 定义按钮尺寸类型
type ButtonSize = "sm" | "md" | "lg" | "icon";

// 组合HTML button元素的所有属性与自定义属性
interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  asChild?: boolean; // 用于支持Slot模式（可选）
}

// 按钮变体的Tailwind CSS类映射
const variantClasses: Record<ButtonVariant, string> = {
  primary: "bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500 active:bg-blue-800",
  secondary: "bg-gray-200 text-gray-800 hover:bg-gray-300 focus:ring-gray-400 active:bg-gray-400",
  outline: "border border-gray-300 bg-transparent text-gray-700 hover:bg-gray-50 focus:ring-gray-400 active:bg-gray-100",
  ghost: "bg-transparent text-gray-700 hover:bg-gray-100 focus:ring-gray-400 active:bg-gray-200",
  destructive: "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500 active:bg-red-800",
  link: "text-blue-600 hover:text-blue-800 underline-offset-4 hover:underline bg-transparent"
};

// 按钮尺寸的Tailwind CSS类映射
const sizeClasses: Record<ButtonSize, string> = {
  sm: "h-8 px-3 text-xs rounded-md",
  md: "h-10 px-4 text-sm rounded-md",
  lg: "h-12 px-6 text-base rounded-lg",
  icon: "h-10 w-10 p-0 rounded-full"
};

/**
 * Button 组件
 * 
 * 一个高度可定制的按钮组件，支持多种变体和尺寸
 * 
 * @param variant - 按钮变体：primary | secondary | outline | ghost | destructive | link
 * @param size - 按钮尺寸：sm | md | lg | icon
 * @param className - 额外的CSS类名
 * @param disabled - 是否禁用按钮
 * @param children - 按钮内容
 * @param rest - 其他HTML button属性
 * 
 * @example
 * ```tsx
 * <Button variant="primary" size="md" onClick={handleClick}>
 *   Click me
 * </Button>
 * ```
 */
const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ 
    className = "", 
    variant = "primary", 
    size = "md", 
    disabled = false,
    children,
    ...props 
  }, ref) => {
    // 基础按钮样式
    const baseClasses = "inline-flex items-center justify-center font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:opacity-50 disabled:pointer-events-none";
    
    // 组合所有样式类
    const combinedClasses = `${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`;

    return (
      <button
        className={combinedClasses}
        disabled={disabled}
        ref={ref}
        {...props}
      >
        {children}
      </button>
    );
  }
);

Button.displayName = "Button";

export type { ButtonProps, ButtonVariant, ButtonSize };
export { Button };