import type React from 'react';

interface MenuProps {
  state: 'edit' | 'view' | 'loading';
  onSave?: () => void;
  onNew: () => void;
  onEdit?: () => void;
  rawUrl?: string | null;
}

const Menu: React.FC<MenuProps> = ({
  state,
  onSave,
  onNew,
  onEdit,
  rawUrl,
}) => {
  const buttonClass =
    'bg-gray-700 hover:bg-gray-600 text-white px-4 py-2 rounded shadow-lg transition-all duration-200 hover:shadow-xl hover:opacity-60 text-sm font-mono';
  const buttonClassMobile =
    'bg-gray-700 hover:bg-gray-600 text-white px-3 py-2 rounded shadow-lg transition-all duration-200 hover:shadow-xl hover:opacity-60 text-xs font-mono';

  return (
    <div className="flex gap-2">
      {state === 'loading' && (
        <button
          type="button"
          className={`${buttonClass} flex items-center gap-2`}
          disabled
        >
          <div className="animate-spin rounded-full h-3 w-3 border border-white border-t-transparent"></div>
          <span className="hidden sm:inline">Loading...</span>
        </button>
      )}

      {state === 'edit' && onSave && (
        <button
          type="button"
          onClick={onSave}
          className={`${buttonClass} hidden sm:block`}
        >
          Save
        </button>
      )}

      {state === 'edit' && onSave && (
        <button
          type="button"
          onClick={onSave}
          className={`${buttonClassMobile} sm:hidden`}
        >
          Save
        </button>
      )}

      {state === 'view' && (
        <>
          {rawUrl && (
            <>
              <a
                href={rawUrl}
                target="_blank"
                rel="noopener noreferrer"
                className={`${buttonClass} hidden sm:block no-underline`}
              >
                Raw
              </a>
              <a
                href={rawUrl}
                target="_blank"
                rel="noopener noreferrer"
                className={`${buttonClassMobile} sm:hidden no-underline`}
              >
                Raw
              </a>
            </>
          )}

          {onEdit && (
            <>
              <button
                type="button"
                onClick={onEdit}
                className={`${buttonClass} hidden sm:block`}
              >
                Edit
              </button>
              <button
                type="button"
                onClick={onEdit}
                className={`${buttonClassMobile} sm:hidden`}
              >
                Edit
              </button>
            </>
          )}

          <button
            type="button"
            onClick={onNew}
            className={`${buttonClass} hidden sm:block`}
          >
            New
          </button>
          <button
            type="button"
            onClick={onNew}
            className={`${buttonClassMobile} sm:hidden`}
          >
            New
          </button>
        </>
      )}
    </div>
  );
};

export default Menu;
