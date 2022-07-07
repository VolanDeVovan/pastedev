import SyntaxHighlighter from 'react-syntax-highlighter';
import { atomOneDark } from 'react-syntax-highlighter/dist/esm/styles/hljs';
import { useLocation, useNavigate, useParams } from "react-router-dom"
import { Menu } from "../components/Menu";
import { MenuButton } from '../components/MenuButton';
import { useEffect, useState } from 'react';
import { API_URL } from '../constants';

interface ViewProps {
    raw?: boolean
}

export const View: React.FC<ViewProps> = ({ raw }) => {
    const { pageId } = useParams()
    const [text, setText] = useState("");

    const location = useLocation()

    const navigate = useNavigate()

    useEffect(() => {
        const fetchText = async () => {
            const resp = await fetch(`${API_URL}/${pageId}`)
            const text = await resp.text()

            if (!resp.ok) throw new Error(`Status code: ${resp.status}`)

            setText(text)
        }

        fetchText().catch(err => navigate('/'))

    }, [pageId])

    const openRaw = () => {
        navigate(location.pathname + '/raw')
    }

    const newEdit = () => {
        navigate('/')
    }

    const forkEdit = () => {
        navigate('/', {
            state: {
                text
            }
        })
    }

    if (raw) {
        return (
            <pre style={{ margin: '0', color: 'white' }}>
                <code>
                    {text}
                </code>
            </pre>
        )
    }

    return (
        <div>
            <Menu>
                <MenuButton onClick={openRaw}>Raw</MenuButton>
                <MenuButton onClick={forkEdit}>Fork</MenuButton>
                <MenuButton onClick={newEdit}>New</MenuButton>
            </Menu>
            <div>
                <SyntaxHighlighter showLineNumbers style={atomOneDark}>
                    {text}
                </SyntaxHighlighter>
            </div>
        </div>
    )
}