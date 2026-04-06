// 测试Button组件的类型定义
import { ButtonProps } from '@/components/ui/Button';

// 验证类型是否正确
const testProps: ButtonProps = {
  variant: 'primary',
  size: 'medium',
  loading: false,
  disabled: false,
  children: 'Test Button',
  onClick: () => console.log('clicked'),
};

export default function TestComponent() {
  return (
    <div>
      <button {...testProps} />
    </div>
  );
}