import cx from 'classnames';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline';
}

/** A small settings button. */
export default function Button({ variant = 'secondary', className, type = 'button', ...props }: Readonly<ButtonProps>) {
  return (
    <button
      type={type}
      className={cx(
        'btn btn-sm',
        {
          'btn-primary': variant === 'primary',
          'btn-secondary': variant === 'secondary',
          'btn-outline': variant === 'outline',
        },
        className,
      )}
      {...props}
    />
  );
}
