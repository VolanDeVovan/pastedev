import styled from "styled-components";

export const MenuButton = styled.button`
    background: none;
    border: none;

    color: white;
    font-family: 'Courier New', Courier, monospace;
    font-size: 20px;

    border-radius: 10px;
    background-color: #282c34;
    box-shadow: 5px 5px 20px -5px black;

    margin-left: 10px;
    user-select: none;

    &:hover {
        color: rgba(255, 255, 255, 0.6);
    }
`
