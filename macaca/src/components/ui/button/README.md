# Button Component Documentation

## Overview
A highly customizable React button component built with TypeScript and Tailwind CSS. Supports multiple variants, sizes, and states while maintaining accessibility standards.

## Installation
This component is designed to work with:
- React 18+
- TypeScript
- Tailwind CSS

Make sure you have Tailwind CSS properly configured in your project.

## Usage

### Basic Usage
```tsx
import { Button } from '@/components/ui/button';

function MyComponent() {
  return (
    <Button onClick={() => console.log('Clicked!')}>
      Click me
    </Button>
  );
}
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `'primary' \| 'secondary' \| 'outline' \| 'ghost' \| 'destructive' \| 'link'` | `'primary'` | Button style variant |
| `size` | `'sm' \| 'md' \| 'lg' \| 'icon'` | `'md'` | Button size |
| `disabled` | `boolean` | `false` | Whether the button is disabled |
| `className` | `string` | `''` | Additional CSS classes |
| `...props` | `React.ButtonHTMLAttributes<HTMLButtonElement>` | - | All standard HTML button attributes |

### Variants

- **primary**: Solid blue background (default action)
- **secondary**: Light gray background (secondary actions)
- **outline**: Transparent with border (minimal emphasis)
- **ghost**: No background, appears as text (subtle actions)
- **destructive**: Red background (delete/dangerous actions)
- **link**: Text-only with underline on hover (navigation)

### Sizes

- **sm**: Small button (8px height, 3px padding)
- **md**: Medium button (10px height, 4px padding) - default
- **lg**: Large button (12px height, 6px padding)
- **icon**: Square button for icons only (10px x 10px)

### Examples

#### Different Variants
```tsx
<Button variant="primary">Primary</Button>
<Button variant="secondary">Secondary</Button>
<Button variant="outline">Outline</Button>
<Button variant="ghost">Ghost</Button>
<Button variant="destructive">Delete</Button>
<Button variant="link">Learn More</Button>
```

#### Different Sizes
```tsx
<Button size="sm">Small</Button>
<Button size="md">Medium</Button>
<Button size="lg">Large</Button>
<Button size="icon">👍</Button>
```

#### Disabled State
```tsx
<Button disabled>Disabled Button</Button>
```

#### With Icons
```tsx
<Button>
  <span className="mr-2">🔍</span>
  Search
</Button>

<Button size="icon">
  ⭐
</Button>
```

## Accessibility Features
- Proper focus management with visible focus rings
- Disabled state properly communicated to screen readers
- Keyboard navigation support
- Sufficient color contrast ratios

## Customization
You can override styles by passing additional classes via the `className` prop:

```tsx
<Button className="bg-purple-600 hover:bg-purple-700">
  Custom Styled Button
</Button>
```

## TypeScript Support
Full TypeScript support with proper type definitions for all props and variants.